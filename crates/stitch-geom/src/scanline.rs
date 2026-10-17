use crate::Polygon;

/// 水平掃描線 `y` 與多邊形相交，回傳被填滿的 x 區間（由左到右，已合併）。
/// 邊的取法為下端含、上端不含，使相鄰區域不會重複或遺漏。
pub fn spans_at(poly: &Polygon, y: f64) -> Vec<(f64, f64)> {
    let mut crossings: Vec<(f64, i32)> = Vec::new();
    for ring in &poly.rings {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if a[1] == b[1] {
                continue;
            }
            let (lo, hi, dir) = if a[1] < b[1] { (a, b, 1) } else { (b, a, -1) };
            if lo[1] <= y && y < hi[1] {
                let t = (y - lo[1]) / (hi[1] - lo[1]);
                crossings.push((lo[0] + t * (hi[0] - lo[0]), dir));
            }
        }
    }
    crossings.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut spans = Vec::new();
    let (mut winding, mut start) = (0, 0.0);
    for (x, dir) in crossings {
        let was = winding;
        winding += dir;
        if was == 0 && winding != 0 {
            start = x;
        } else if was != 0 && winding == 0 && x > start {
            spans.push((start, x));
        }
    }
    spans
}
