use crate::color::{delta, lab, Rgb};
use crate::quantize::quantize;
use crate::trace::{label_polygon, majority_filter};
use serde::{Deserialize, Serialize};
use stitch_core::hoop::Hoop;
use stitch_core::palette::threads;
use stitch_core::{ThreadColor, UserMessage};
use stitch_gen::design::{build_plan, ColoredShape, PreviewResult, Settings};

/// 處理時圖片長邊最多幾像素（再大的先縮小，速度與品質都夠用）。
const MAX_SIDE: usize = 256;
/// 平均色差超過這個值，代表漸層很多（像照片），效果有限。
const PHOTO_ERROR: f64 = 10.0;
/// 簡化與平滑後，被去掉的面積超過這個比例就說明。
const SIMPLIFIED_NOTE: f64 = 0.03;
/// 與背景色差小於這個值的顏色，視為背景的一部分。
const NEAR_BACKGROUND: f64 = 20.0;
/// 亮度（Lab 的 L）低於這個值才算深色線。
/// 兩個顏色的色差小於這個值就合併成一個。
const MERGE_DELTA: f64 = 12.0;
const DARK_LINE_L: f64 = 50.0;

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

/// 色差很小的群合併成一群（顏色數抓得比實際多時，同一個色塊會被切成兩三個幾乎一樣的色）。
/// 回傳合併後的各群中心；`labels` 會改成新的編號。
fn merge_close(labels: &mut [i16], centers: &[Rgb]) -> Vec<Rgb> {
    let mut weight = vec![0.0f64; centers.len()];
    for &l in labels.iter().filter(|l| **l >= 0) {
        weight[l as usize] += 1.0;
    }
    let mut group: Vec<usize> = (0..centers.len()).collect();
    let mut c: Vec<[f64; 3]> = centers
        .iter()
        .map(|&r| [r[0] as f64, r[1] as f64, r[2] as f64])
        .collect();
    loop {
        let live: Vec<usize> = (0..c.len())
            .filter(|&i| group[i] == i && weight[i] > 0.0)
            .collect();
        let mut best: Option<(f64, usize, usize)> = None;
        for (ai, &a) in live.iter().enumerate() {
            for &b in &live[ai + 1..] {
                let d = delta(
                    lab([c[a][0] as u8, c[a][1] as u8, c[a][2] as u8]),
                    lab([c[b][0] as u8, c[b][1] as u8, c[b][2] as u8]),
                );
                if d < MERGE_DELTA && best.is_none_or(|x| d < x.0) {
                    best = Some((d, a, b));
                }
            }
        }
        let Some((_, a, b)) = best else { break };
        // 把 b 併進 a（中心取加權平均）
        let cb = c[b];
        let (wa, wb) = (weight[a], weight[b]);
        for (v, vb) in c[a].iter_mut().zip(cb) {
            *v = (*v * wa + vb * wb) / (wa + wb);
        }
        weight[a] = wa + wb;
        weight[b] = 0.0;
        group[b] = a;
    }
    let mut new_index = vec![-1i16; centers.len()];
    let mut out: Vec<Rgb> = Vec::new();
    for i in 0..centers.len() {
        if group[i] == i && weight[i] > 0.0 {
            new_index[i] = out.len() as i16;
            out.push([
                c[i][0].round() as u8,
                c[i][1].round() as u8,
                c[i][2].round() as u8,
            ]);
        }
    }
    for l in labels.iter_mut().filter(|l| **l >= 0) {
        let mut g = *l as usize;
        while group[g] != g {
            g = group[g];
        }
        *l = new_index[g];
    }
    out
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
    let centers = merge_close(&mut labels, &q.centers);

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
            // 顏色跟背景幾乎一樣的（圖案邊緣的過渡色）一併當背景，免得繡出一圈多餘的邊
            let bg_lab = lab(centers[bg as usize]);
            let near_bg: Vec<bool> = centers
                .iter()
                .enumerate()
                .map(|(i, &c)| i as i16 == bg || delta(lab(c), bg_lab) < NEAR_BACKGROUND)
                .collect();
            for l in labels
                .iter_mut()
                .filter(|l| **l >= 0 && near_bg[**l as usize])
            {
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
    // 平滑半徑以「一個像素有多大」為準：磨掉像素階梯，又不會把細節整個吃掉
    let smooth_r = scale * (1.0 + simplify / 100.0);
    let min_feature = req.settings.min_stroke_width * (1.0 + simplify / 50.0);
    let palette = threads();
    let mut shapes: Vec<(f64, ColoredShape)> = Vec::new();
    let mut area_px = 0.0;
    for (label, &center) in centers.iter().enumerate() {
        let poly = label_polygon(&labels, w, h, label as i16);
        if poly.is_empty() {
            continue;
        }
        let phys = poly
            .translate(-(w as f64) / 2.0, -(h as f64) / 2.0)
            .scale(scale);
        let smoothed = phys.offset(smooth_r).offset(-smooth_r);
        // 深色的部分（卡通圖的輪廓、眼睛、文字）：細的地方整條去掉會讓圖案糊成一團，
        // 所以不做「去掉太細的部分」，改成整體稍微加粗，讓細線剛好繡得出來。
        // 淺色的細線只是邊緣的模糊過渡，照常去掉，否則圖案外面會多一圈邊。
        let cleaned = if lab(center)[0] < DARK_LINE_L {
            let eps = min_feature * 0.1;
            smoothed.offset(-eps).offset(eps + min_feature / 4.0)
        } else {
            smoothed
                .offset(-min_feature / 2.0)
                .offset(min_feature / 2.0)
        };
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
    if lost.abs() > SIMPLIFIED_NOTE {
        notes.push(UserMessage::new(
            "simplified",
            "有些地方太細，繡出來會糊，已幫你加粗、合併或去掉了。",
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
