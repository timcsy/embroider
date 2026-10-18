//! 重要性地圖：估計「人看這張圖時，哪裡會被注意到」。
//! 做法是古典的 Frequency-tuned 顯著性（Achanta 等人，CVPR 2009）：稍微模糊後的 Lab 顏色
//! 與全圖平均色的距離，越不一樣越顯眼；再乘上「中心偏好」（人看圖偏愛看中間，Judd 等人 2009）。
//! 不用模型，純函式，相同輸入得到相同輸出。

use crate::color::{delta, lab, Lab};

/// 色差的下限：整張圖最大的差異還不到這個值，就當作沒有重點，免得把雜訊放大成重點。
const MIN_CONTRAST: f64 = 20.0;
/// 中心偏好的寬度（相對於半個畫面）。
const CENTER_SIGMA: f64 = 0.5;
/// 角落最低保留的比例；畫面邊緣的重要東西（例如邊框、標題）不能完全被忽略。
const CENTER_FLOOR: f64 = 0.5;
const BLUR_RADIUS: usize = 1;
/// 顯著性取鄰近的最大值的範圍：細字、細線模糊後對比會被稀釋，但它們一樣顯眼。
const PEAK_RADIUS: usize = 2;

fn box_blur(src: &[f64], w: usize, h: usize, r: usize) -> Vec<f64> {
    let pass = |src: &[f64], horizontal: bool| -> Vec<f64> {
        let mut out = vec![0.0; src.len()];
        for y in 0..h {
            for x in 0..w {
                let (mut sum, mut n) = (0.0, 0.0);
                for d in 0..=2 * r {
                    let (xx, yy) = if horizontal {
                        ((x + d).checked_sub(r), Some(y))
                    } else {
                        (Some(x), (y + d).checked_sub(r))
                    };
                    if let (Some(xx), Some(yy)) = (xx, yy) {
                        if xx < w && yy < h {
                            sum += src[yy * w + xx];
                            n += 1.0;
                        }
                    }
                }
                out[y * w + x] = sum / n;
            }
        }
        out
    };
    pass(&pass(src, true), false)
}

/// 取各像素周圍（方形）的最大值。
fn peak_filter(src: &[f64], w: usize, h: usize, r: usize) -> Vec<f64> {
    let pass = |src: &[f64], horizontal: bool| -> Vec<f64> {
        (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                (0..=2 * r)
                    .filter_map(|d| {
                        let (xx, yy) = if horizontal {
                            ((x + d).checked_sub(r)?, y)
                        } else {
                            (x, (y + d).checked_sub(r)?)
                        };
                        (xx < w && yy < h).then(|| src[yy * w + xx])
                    })
                    .fold(0.0, f64::max)
            })
            .collect()
    };
    pass(&pass(src, true), false)
}

/// 每個像素的重要性（0 到 1）。透明的地方一律是 0。
pub fn importance_map(rgba: &[u8], w: usize, h: usize) -> Vec<f64> {
    let n = w * h;
    let opaque: Vec<bool> = (0..n).map(|i| rgba[i * 4 + 3] >= 128).collect();
    let labs: Vec<Lab> = (0..n)
        .map(|i| lab([rgba[i * 4], rgba[i * 4 + 1], rgba[i * 4 + 2]]))
        .collect();
    let count = opaque.iter().filter(|&&o| o).count();
    if count == 0 {
        return vec![0.0; n];
    }
    let mut mean = [0.0; 3];
    for i in (0..n).filter(|&i| opaque[i]) {
        for k in 0..3 {
            mean[k] += labs[i][k] / count as f64;
        }
    }
    // 透明處用平均色補，模糊時才不會被拉向黑色
    let channel = |k: usize| -> Vec<f64> {
        let raw: Vec<f64> = (0..n)
            .map(|i| if opaque[i] { labs[i][k] } else { mean[k] })
            .collect();
        box_blur(&raw, w, h, BLUR_RADIUS)
    };
    let (l, a, b) = (channel(0), channel(1), channel(2));
    let contrast: Vec<f64> = (0..n)
        .map(|i| {
            if opaque[i] {
                delta([l[i], a[i], b[i]], mean)
            } else {
                0.0
            }
        })
        .collect();
    let contrast = peak_filter(&contrast, w, h, PEAK_RADIUS);
    let top = contrast
        .iter()
        .cloned()
        .fold(0.0, f64::max)
        .max(MIN_CONTRAST);
    (0..n)
        .map(|i| {
            if !opaque[i] {
                return 0.0;
            }
            let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
            let (u, v) = (
                (x - w as f64 / 2.0) / (w as f64 / 2.0),
                (y - h as f64 / 2.0) / (h as f64 / 2.0),
            );
            let center = (-(u * u + v * v) / (2.0 * CENTER_SIGMA * CENTER_SIGMA)).exp();
            let prior = CENTER_FLOOR + (1.0 - CENTER_FLOOR) * center;
            (contrast[i] / top * prior).clamp(0.0, 1.0)
        })
        .collect()
}
