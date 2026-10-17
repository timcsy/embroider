use crate::fill::{fill, FillParams};
use crate::outline::outline;
use crate::Run;
use stitch_geom::Polygon;

/// 底針內縮量（0.3mm）。
pub const INSET: f64 = 3.0;

/// 底針：內縮後沿邊走一圈，再以垂直於面針的方向鋪一層稀疏的針跡。
/// 區域太小（內縮後消失）時沒有底針。
pub fn underlay(poly: &Polygon, fill_angle_deg: f64) -> Vec<Run> {
    let inner = poly.offset(-INSET);
    if inner.is_empty() {
        return Vec::new();
    }
    let mut runs = outline(&inner, 25.0);
    runs.extend(fill(
        &inner,
        &FillParams {
            row_spacing: 24.0,
            max_stitch: 35.0,
            min_stitch: 5.0,
            angle_deg: fill_angle_deg + 90.0,
        },
    ));
    runs
}
