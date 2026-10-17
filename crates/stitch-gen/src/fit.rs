use stitch_core::hoop::{Hoop, HoopShape};
use stitch_core::UserMessage;
use stitch_geom::Polygon;

#[derive(Debug, Clone, PartialEq)]
pub enum FitResult {
    /// 放得下，不需縮放
    Fits,
    /// 需要等比縮小到這個倍率才放得下
    Scaled(f64),
    /// 縮小到可接受下限仍放不下
    Blocked(UserMessage),
}

/// 預留給取整為整數座標的餘裕（0.1mm）。
const ROUNDING_MARGIN: f64 = 1.0;

/// 判斷以框中心為原點的多邊形是否放得進框的安全範圍；放不下時算出需要的縮放倍率。
/// 框是凸形，所以頂點都在範圍內，整個多邊形就在範圍內。
/// `min_scale` 是可接受的最小縮放倍率，低於它就阻止。
pub fn fit_polygon(poly: &Polygon, hoop: &Hoop, min_scale: f64) -> FitResult {
    let hw = (hoop.width - 2 * hoop.margin) as f64 / 2.0 - ROUNDING_MARGIN;
    let hh = (hoop.height - 2 * hoop.margin) as f64 / 2.0 - ROUNDING_MARGIN;
    let blocked = || {
        FitResult::Blocked(UserMessage::new(
            "too_big_for_hoop",
            &format!(
                "這個設計放不進「{}」。縮小的話字會太小，繡出來會糊。",
                hoop.name
            ),
            Some("請換一個比較大的框，或減少字數。"),
        ))
    };
    if hw <= 0.0 || hh <= 0.0 {
        return blocked();
    }
    let mut scale = f64::INFINITY;
    for p in poly.rings.iter().flatten() {
        let s = match hoop.shape {
            HoopShape::Rect => {
                let sx = if p[0] == 0.0 {
                    f64::INFINITY
                } else {
                    hw / p[0].abs()
                };
                let sy = if p[1] == 0.0 {
                    f64::INFINITY
                } else {
                    hh / p[1].abs()
                };
                sx.min(sy)
            }
            HoopShape::Circle | HoopShape::Ellipse => {
                let r = ((p[0] / hw).powi(2) + (p[1] / hh).powi(2)).sqrt();
                if r == 0.0 {
                    f64::INFINITY
                } else {
                    1.0 / r
                }
            }
        };
        scale = scale.min(s);
    }
    if scale >= 1.0 {
        FitResult::Fits
    } else if scale < min_scale {
        blocked()
    } else {
        FitResult::Scaled(scale)
    }
}
