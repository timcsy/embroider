//! 保邊平滑（雙邊濾波）：平坦區域的雜訊被抹平，明顯的邊緣保留。
//! 照片與 JPEG 圖直接分群會得到一堆零碎的小色塊；先保邊平滑，同一個色塊的顏色才會一致，
//! 分群出來的區域才大而完整（業界的自動數位化也是先做保邊低通濾波再分群）。

/// 空間範圍（像素）。
const RADIUS: i32 = 3;
/// 空間權重的標準差（像素）。
const SIGMA_SPACE: f64 = 2.0;

/// `strength`：0～100。0 原樣回傳；越大，越大的顏色差異也會被當成「同一塊」而抹平。
/// 透明的像素不改，也不參與鄰居的平均。
pub fn edge_preserving_smooth(rgba: &[u8], w: usize, h: usize, strength: u32) -> Vec<u8> {
    if strength == 0 || w == 0 || h == 0 {
        return rgba.to_vec();
    }
    let sigma_range = 12.0 + strength.min(100) as f64 * 0.5; // RGB 距離
    let mut cur = rgba.to_vec();
    for _ in 0..2 {
        cur = pass(&cur, w, h, sigma_range);
    }
    cur
}

fn pass(src: &[u8], w: usize, h: usize, sigma_range: f64) -> Vec<u8> {
    let mut out = src.to_vec();
    let inv_r = -1.0 / (2.0 * sigma_range * sigma_range);
    let inv_s = -1.0 / (2.0 * SIGMA_SPACE * SIGMA_SPACE);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            if src[i + 3] < 128 {
                continue;
            }
            let c = [src[i] as f64, src[i + 1] as f64, src[i + 2] as f64];
            let (mut sum, mut wsum) = ([0.0f64; 3], 0.0f64);
            for dy in -RADIUS..=RADIUS {
                let ny = y as i32 + dy;
                if ny < 0 || ny >= h as i32 {
                    continue;
                }
                for dx in -RADIUS..=RADIUS {
                    let nx = x as i32 + dx;
                    if nx < 0 || nx >= w as i32 {
                        continue;
                    }
                    let j = (ny as usize * w + nx as usize) * 4;
                    if src[j + 3] < 128 {
                        continue;
                    }
                    let d = [
                        src[j] as f64 - c[0],
                        src[j + 1] as f64 - c[1],
                        src[j + 2] as f64 - c[2],
                    ];
                    let dist2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
                    let weight = (inv_r * dist2 + inv_s * (dx * dx + dy * dy) as f64).exp();
                    wsum += weight;
                    for k in 0..3 {
                        sum[k] += weight * src[j + k] as f64;
                    }
                }
            }
            for k in 0..3 {
                out[i + k] = (sum[k] / wsum).round() as u8;
            }
        }
    }
    out
}
