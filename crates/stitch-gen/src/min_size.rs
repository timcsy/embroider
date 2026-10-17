use stitch_geom::Polygon;

/// 最小可繡尺寸，單位 0.1mm。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MinSizeParams {
    /// 最小字高（預設 40 = 4mm）
    pub min_text_height: f64,
    /// 最小筆畫寬（預設 8 = 0.8mm）
    pub min_stroke_width: f64,
}

impl Default for MinSizeParams {
    fn default() -> Self {
        Self {
            min_text_height: 40.0,
            min_stroke_width: 8.0,
        }
    }
}

/// 為了讓筆畫夠寬，最多放大幾倍。
pub const MAX_STROKE_SCALE: f64 = 3.0;
/// 容許被判為「太細」而消失的面積比例。
const THIN_TOLERANCE: f64 = 0.03;

/// 太細的部分占整體面積的比例（形態學開運算：先內縮半個筆寬、再外擴回來，消失的就是太細的）。
fn thin_fraction(poly: &Polygon, min_stroke: f64) -> f64 {
    let area = poly.area();
    if area <= 0.0 {
        return 0.0;
    }
    let r = min_stroke / 2.0;
    let inner = poly.offset(-r);
    if inner.is_empty() {
        return 1.0;
    }
    let kept = inner.offset(r).area();
    ((area - kept) / area).max(0.0)
}

fn ok_at(poly: &Polygon, k: f64, p: &MinSizeParams) -> bool {
    thin_fraction(&poly.scale(k), p.min_stroke_width) <= THIN_TOLERANCE
}

/// 為了「繡得出來」至少要放大幾倍（不小於 1）。取「達到最小字高」與「筆畫夠寬」中較大的；
/// 筆畫放大上限為 [`MAX_STROKE_SCALE`] 倍。多數情況只要檢查一次（目前的大小就夠了）。
pub fn enlarge_needed(poly: &Polygon, text_height: f64, p: &MinSizeParams) -> f64 {
    let start = (p.min_text_height / text_height).max(1.0);
    if ok_at(poly, start, p) || start >= MAX_STROKE_SCALE {
        return start;
    }
    if !ok_at(poly, MAX_STROKE_SCALE, p) {
        return MAX_STROKE_SCALE;
    }
    let (mut lo, mut hi) = (start, MAX_STROKE_SCALE);
    for _ in 0..8 {
        let mid = (lo + hi) / 2.0;
        if ok_at(poly, mid, p) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    hi
}

/// 從 `from`（已知夠大）往下找，最多可以縮到幾倍還繡得出來。
/// 下限是最小字高與筆畫夠寬兩者中較嚴的。只在真的需要縮小時才計算。
pub fn shrink_limit(poly: &Polygon, from: f64, text_height: f64, p: &MinSizeParams) -> f64 {
    let floor = p.min_text_height / text_height;
    if from <= floor {
        return from.max(floor);
    }
    // 一路往下縮 15%，直到不夠大或碰到字高下限，再在最後一段內二分
    let mut ok = from;
    let mut lo = loop {
        let next = ok * 0.85;
        if next <= floor {
            if ok_at(poly, floor, p) {
                return floor;
            }
            break floor;
        }
        if ok_at(poly, next, p) {
            ok = next;
        } else {
            break next;
        }
    };
    for _ in 0..4 {
        let mid = (lo + ok) / 2.0;
        if ok_at(poly, mid, p) {
            ok = mid;
        } else {
            lo = mid;
        }
    }
    ok
}

/// 讓這個設計「剛好夠大、繡得出來」所需的縮放倍率（相對於目前大小，可能小於 1）。
pub fn min_scale_needed(poly: &Polygon, text_height: f64, p: &MinSizeParams) -> f64 {
    let up = enlarge_needed(poly, text_height, p);
    if up > 1.0 {
        up
    } else {
        shrink_limit(poly, 1.0, text_height, p)
    }
}
