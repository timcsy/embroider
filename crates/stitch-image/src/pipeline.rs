use crate::color::{delta, lab, Rgb};
use crate::quantize::quantize;
use crate::trace::{label_polygon, majority_filter};
use serde::{Deserialize, Serialize};
use stitch_core::hoop::Hoop;
use stitch_core::palette::threads;
use stitch_core::{ThreadColor, UserMessage};
use stitch_gen::design::{build_plan, ColoredShape, PreviewResult, Settings};

/// 處理時圖片長邊最多幾像素（再大的先縮小，速度與品質都夠用）。
const MAX_SIDE: usize = 160;
/// 平均色差超過這個值，代表漸層很多（像照片），效果有限。
const PHOTO_ERROR: f64 = 10.0;
/// 簡化與平滑後，被去掉的面積超過這個比例就說明。
const SIMPLIFIED_NOTE: f64 = 0.03;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageRequest {
    /// 解碼後的 RGBA 像素（由呼叫端另外傳入，不經過 JSON）
    #[serde(default, skip_serializing)]
    pub rgba: Vec<u8>,
    pub width: usize,
    pub height: usize,
    /// 顏色多寡（2～12）
    pub color_count: usize,
    /// 簡化程度（0～100）
    pub simplify: u32,
    /// 圖長邊的大小（0.1mm）
    pub size: f64,
    pub hoop: Hoop,
    #[serde(default)]
    pub settings: Settings,
}

fn blocked(code: &str, text: &str, hint: &str) -> PreviewResult {
    PreviewResult::blocked(UserMessage::new(code, text, Some(hint)))
}

/// 長邊超過 `MAX_SIDE` 時以方塊平均縮小；一半以上透明的方塊視為透明。
fn downscale(rgba: &[u8], w: usize, h: usize) -> (Vec<u8>, usize, usize) {
    let long = w.max(h);
    if long <= MAX_SIDE {
        return (rgba.to_vec(), w, h);
    }
    let f = long.div_ceil(MAX_SIDE);
    let (nw, nh) = (w.div_ceil(f), h.div_ceil(f));
    let mut out = Vec::with_capacity(nw * nh * 4);
    for by in 0..nh {
        for bx in 0..nw {
            let (mut sum, mut opaque, mut total) = ([0u32; 3], 0u32, 0u32);
            for y in by * f..((by + 1) * f).min(h) {
                for x in bx * f..((bx + 1) * f).min(w) {
                    let p = &rgba[(y * w + x) * 4..(y * w + x) * 4 + 4];
                    total += 1;
                    if p[3] >= 128 {
                        opaque += 1;
                        for c in 0..3 {
                            sum[c] += p[c] as u32;
                        }
                    }
                }
            }
            if opaque * 2 >= total && opaque > 0 {
                out.extend_from_slice(&[
                    (sum[0] / opaque) as u8,
                    (sum[1] / opaque) as u8,
                    (sum[2] / opaque) as u8,
                    255,
                ]);
            } else {
                out.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    (out, nw, nh)
}

fn nearest_thread(c: Rgb, palette: &[ThreadColor]) -> ThreadColor {
    let l = lab(c);
    palette
        .iter()
        .min_by(|a, b| {
            delta(l, lab([a.r, a.g, a.b]))
                .partial_cmp(&delta(l, lab([b.r, b.g, b.b])))
                .unwrap()
        })
        .unwrap()
        .clone()
}

/// 圖片 → 針跡計畫。任何無法輸出的情況都以 `blocked` 回報白話原因。
pub fn build_image_preview(req: &ImageRequest) -> PreviewResult {
    let simplify = req.simplify.min(100) as f64;
    let (rgba, w, h) = downscale(&req.rgba, req.width, req.height);
    if rgba.len() != req.rgba.len() && req.rgba.len() != req.width * req.height * 4 {
        return blocked("bad_image", "這張圖打不開。", "請換一張圖，或重新選一次。");
    }

    let q = match quantize(&rgba, w, h, req.color_count) {
        Ok(q) => q,
        Err(m) => return PreviewResult::blocked(m),
    };
    let mut labels = q.labels.clone();

    // 背景：圖片邊緣最多的那個顏色（例如白底）不繡
    let mut border: Vec<(i16, usize)> = Vec::new();
    let mut border_total = 0usize;
    for y in 0..h {
        for x in 0..w {
            if x != 0 && y != 0 && x != w - 1 && y != h - 1 {
                continue;
            }
            let l = labels[y * w + x];
            if l < 0 {
                continue;
            }
            border_total += 1;
            match border.iter_mut().find(|b| b.0 == l) {
                Some(b) => b.1 += 1,
                None => border.push((l, 1)),
            }
        }
    }
    if let Some(&(bg, n)) = border.iter().max_by_key(|b| (b.1, std::cmp::Reverse(b.0))) {
        if n * 2 >= border_total {
            for l in labels.iter_mut().filter(|l| **l == bg) {
                *l = -1;
            }
        }
    }
    let before = labels.iter().filter(|&&l| l >= 0).count();
    if before == 0 {
        return blocked(
            "single_color",
            "這張圖幾乎只有一種顏色，沒有可以繡出圖案的地方。",
            "請換一張有好幾種顏色、圖案分明的圖。",
        );
    }

    majority_filter(&mut labels, w, h, 1 + (simplify / 40.0) as usize);

    // 逐色轉成多邊形，換成實際大小（0.1mm），再依簡化程度平滑、合併過細的部分
    let scale = req.size / w.max(h) as f64;
    let smooth_r = 1.5 + simplify * 0.1;
    let min_feature = req.settings.min_stroke_width * (1.0 + simplify / 50.0);
    let palette = threads();
    let mut shapes: Vec<(f64, ColoredShape)> = Vec::new();
    let mut area_px = 0.0;
    for (label, &center) in q.centers.iter().enumerate() {
        let poly = label_polygon(&labels, w, h, label as i16);
        if poly.is_empty() {
            continue;
        }
        let phys = poly
            .translate(-(w as f64) / 2.0, -(h as f64) / 2.0)
            .scale(scale);
        let cleaned = phys
            .offset(smooth_r)
            .offset(-smooth_r)
            .offset(-min_feature / 2.0)
            .offset(min_feature / 2.0);
        if cleaned.is_empty() {
            continue;
        }
        area_px += cleaned.area() / (scale * scale);
        shapes.push((
            cleaned.area(),
            ColoredShape {
                polygon: cleaned,
                color: nearest_thread(center, &palette),
            },
        ));
    }
    if shapes.is_empty() {
        return blocked(
            "nothing_left",
            "圖案太細碎了，簡化之後什麼都沒剩下。",
            "請把「簡化程度」調低一點，或把圖放大一點。",
        );
    }
    // 大面積先繡，細節疊在上面
    shapes.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let shapes: Vec<ColoredShape> = shapes.into_iter().map(|s| s.1).collect();

    let mut notes: Vec<UserMessage> = Vec::new();
    let significant = q.mean_error > PHOTO_ERROR;
    if significant {
        notes.push(UserMessage::new(
            "photo_limited",
            "這張圖的顏色很多、漸層也多，繡出來效果有限。",
            Some("色塊分明的圖案（例如卡通、標誌）會比較好看。你還是可以繼續。"),
        ));
    }
    let lost = 1.0 - area_px / before as f64;
    if lost > SIMPLIFIED_NOTE {
        notes.push(UserMessage::new(
            "simplified",
            "有些地方太細，繡出來會糊，已幫你合併或去掉了。",
            Some("想保留更多細節，可以把「簡化程度」調低，或讓圖片大一點。"),
        ));
    }

    let mut result = build_plan(&shapes, req.size, &req.hoop, &req.settings);
    if result.blocked.is_none() {
        notes.append(&mut result.adjustments);
        result.adjustments = notes;
    }
    result
}
