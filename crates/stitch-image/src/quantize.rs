//! 色彩分群（k-means，在 Lab 色彩空間）。結果是固定的：同樣的圖與顏色數永遠得到同樣的分群。

use crate::color::{delta, lab, Lab, Rgb};
use std::collections::BTreeMap;
use stitch_core::UserMessage;

pub const MIN_COLORS: usize = 2;
pub const MAX_COLORS: usize = 12;
/// alpha 小於這個值的像素視為透明
const OPAQUE_ALPHA: u8 = 128;

#[derive(Debug, Clone, PartialEq)]
pub struct Quantized {
    pub width: usize,
    pub height: usize,
    /// 每個像素的群編號；透明為 -1
    pub labels: Vec<i16>,
    pub centers: Vec<Rgb>,
    /// 像素與所屬群中心的平均色差（Lab 距離）。平坦色塊接近 0，照片通常很大。
    pub mean_error: f64,
}

fn err(code: &str, text: &str, hint: &str) -> UserMessage {
    UserMessage::new(code, text, Some(hint))
}

fn check_size(rgba: &[u8], w: usize, h: usize) -> Result<(), UserMessage> {
    if w == 0 || h == 0 || rgba.len() != w * h * 4 {
        return Err(err(
            "bad_image",
            "這張圖打不開。",
            "請換一張圖，或重新選一次。",
        ));
    }
    Ok(())
}

pub fn quantize(rgba: &[u8], w: usize, h: usize, k: usize) -> Result<Quantized, UserMessage> {
    if !(MIN_COLORS..=MAX_COLORS).contains(&k) {
        return Err(err(
            "bad_color_count",
            "顏色的數量要在 2 到 12 之間。",
            "請把「顏色多寡」調到中間一點。",
        ));
    }
    check_size(rgba, w, h)?;

    // 不重複的顏色與出現次數（BTreeMap 讓順序固定）
    let mut hist: BTreeMap<Rgb, u32> = BTreeMap::new();
    for px in rgba.chunks_exact(4) {
        if px[3] >= OPAQUE_ALPHA {
            *hist.entry([px[0], px[1], px[2]]).or_insert(0) += 1;
        }
    }
    if hist.is_empty() {
        return Err(err(
            "empty_image",
            "這張圖是全透明的，沒有東西可以繡。",
            "請換一張有顏色的圖。",
        ));
    }
    let colors: Vec<(Rgb, Lab, f64)> = hist.iter().map(|(&c, &n)| (c, lab(c), n as f64)).collect();
    let k = k.min(colors.len());

    // 初始中心：由最常出現的顏色開始，每次挑離現有中心最遠的（farthest-first）
    let first = (0..colors.len())
        .max_by(|&a, &b| colors[a].2.partial_cmp(&colors[b].2).unwrap())
        .unwrap();
    let mut centers: Vec<Lab> = vec![colors[first].1];
    while centers.len() < k {
        let far = (0..colors.len())
            .max_by(|&a, &b| {
                let da = centers
                    .iter()
                    .map(|&c| delta(c, colors[a].1))
                    .fold(f64::MAX, f64::min);
                let db = centers
                    .iter()
                    .map(|&c| delta(c, colors[b].1))
                    .fold(f64::MAX, f64::min);
                da.partial_cmp(&db).unwrap()
            })
            .unwrap();
        centers.push(colors[far].1);
    }

    let nearest = |centers: &[Lab], p: Lab| -> usize {
        (0..centers.len())
            .min_by(|&a, &b| {
                delta(centers[a], p)
                    .partial_cmp(&delta(centers[b], p))
                    .unwrap()
            })
            .unwrap()
    };

    let mut assign: Vec<usize> = vec![0; colors.len()];
    for _ in 0..30 {
        let next: Vec<usize> = colors.iter().map(|c| nearest(&centers, c.1)).collect();
        let stable = next == assign;
        assign = next;
        let mut sum = vec![[0.0f64; 3]; k];
        let mut weight = vec![0.0f64; k];
        for (i, c) in colors.iter().enumerate() {
            let a = assign[i];
            weight[a] += c.2;
            for (acc, v) in sum[a].iter_mut().zip(c.1) {
                *acc += v * c.2;
            }
        }
        for a in 0..k {
            if weight[a] > 0.0 {
                centers[a] = [
                    sum[a][0] / weight[a],
                    sum[a][1] / weight[a],
                    sum[a][2] / weight[a],
                ];
            }
        }
        if stable {
            break;
        }
    }
    assign = colors.iter().map(|c| nearest(&centers, c.1)).collect();

    // 每群的代表色：成員的 RGB 加權平均（比從 Lab 反推更貼近原圖）；空的群丟掉
    let mut rgb_sum = vec![[0.0f64; 3]; k];
    let mut weight = vec![0.0f64; k];
    for (i, c) in colors.iter().enumerate() {
        weight[assign[i]] += c.2;
        for (acc, v) in rgb_sum[assign[i]].iter_mut().zip(c.0) {
            *acc += v as f64 * c.2;
        }
    }
    let mut remap = vec![usize::MAX; k];
    let mut out_centers: Vec<Rgb> = Vec::new();
    for a in 0..k {
        if weight[a] > 0.0 {
            remap[a] = out_centers.len();
            let m = |d: usize| (rgb_sum[a][d] / weight[a]).round() as u8;
            out_centers.push([m(0), m(1), m(2)]);
        }
    }
    let label_of: BTreeMap<Rgb, i16> = colors
        .iter()
        .enumerate()
        .map(|(i, c)| (c.0, remap[assign[i]] as i16))
        .collect();

    let center_lab: Vec<Lab> = out_centers.iter().map(|&c| lab(c)).collect();
    let mut err_sum = 0.0;
    let mut n = 0.0;
    for c in &colors {
        let l = label_of[&c.0] as usize;
        err_sum += delta(c.1, center_lab[l]) * c.2;
        n += c.2;
    }

    let labels = rgba
        .chunks_exact(4)
        .map(|px| {
            if px[3] >= OPAQUE_ALPHA {
                label_of[&[px[0], px[1], px[2]]]
            } else {
                -1
            }
        })
        .collect();

    Ok(Quantized {
        width: w,
        height: h,
        labels,
        centers: out_centers,
        mean_error: err_sum / n,
    })
}

/// 依圖片內容建議顏色數：看有幾種顏色各占 2% 以上，限制在 2 到 8 之間。
pub fn suggest_color_count(rgba: &[u8], w: usize, h: usize) -> usize {
    if check_size(rgba, w, h).is_err() {
        return MIN_COLORS;
    }
    let mut bins: BTreeMap<[u8; 3], u32> = BTreeMap::new();
    let mut opaque = 0u32;
    for px in rgba.chunks_exact(4) {
        if px[3] >= OPAQUE_ALPHA {
            *bins
                .entry([px[0] >> 4, px[1] >> 4, px[2] >> 4])
                .or_insert(0) += 1;
            opaque += 1;
        }
    }
    let significant = bins
        .values()
        .filter(|&&n| n as f64 >= opaque as f64 * 0.02)
        .count();
    significant.clamp(MIN_COLORS, 8)
}
