use crate::Run;
use stitch_geom::{spans_at, Polygon};

/// 填充（tatami）參數，單位 0.1mm。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillParams {
    /// 行距（預設 4 = 0.4mm）
    pub row_spacing: f64,
    /// 單針最大長度（預設 30 = 3mm）
    pub max_stitch: f64,
    /// 單針最短長度；更短的針點會被合併（預設 5 = 0.5mm）
    pub min_stitch: f64,
    /// 填充方向（度）
    pub angle_deg: f64,
    /// 第一列往下錯開多少（預設 0）。兩色交錯混色時，第二色錯開一個行距，兩色的列才會交錯。
    pub y_offset: f64,
}

impl Default for FillParams {
    fn default() -> Self {
        Self {
            row_spacing: 4.0,
            max_stitch: 30.0,
            min_stitch: 5.0,
            angle_deg: 45.0,
            y_offset: 0.0,
        }
    }
}

fn rotate(p: [f64; 2], deg: f64) -> [f64; 2] {
    if deg == 0.0 {
        return p;
    }
    let (s, c) = deg.to_radians().sin_cos();
    [p[0] * c - p[1] * s, p[0] * s + p[1] * c]
}

fn rotate_poly(poly: &Polygon, deg: f64) -> Polygon {
    Polygon::new(
        poly.rings
            .iter()
            .map(|r| r.iter().map(|&p| rotate(p, deg)).collect())
            .collect(),
    )
}

/// 一列區段內的 x 位置：兩端加上錯開的格點，過密的點合併，且保證每針不超過上限。
fn row_points(x0: f64, x1: f64, offset: f64, p: &FillParams) -> Vec<f64> {
    let l = p.max_stitch;
    let mut pts = vec![x0];
    let mut n = ((x0 - offset) / l).floor();
    loop {
        let g = offset + n * l;
        n += 1.0;
        if g <= x0 + 1e-9 {
            continue;
        }
        if g >= x1 - 1e-9 {
            break;
        }
        pts.push(g);
    }
    pts.push(x1);

    let mut kept = vec![pts[0]];
    for i in 1..pts.len() - 1 {
        let (cur, next, last) = (pts[i], pts[i + 1], *kept.last().unwrap());
        let too_close_before = cur - last < p.min_stitch && next - last <= l;
        let too_close_to_end =
            i + 1 == pts.len() - 1 && next - cur < p.min_stitch && next - last <= l;
        if !(too_close_before || too_close_to_end) {
            kept.push(cur);
        }
    }
    kept.push(*pts.last().unwrap());
    kept
}

struct Strip {
    /// (列號, 區段)
    rows: Vec<(usize, (f64, f64))>,
}

/// 把各列的區段串成「條」：下一列中與條尾端區段重疊的區段接在同一條上，
/// 這樣同一條筆畫會連續縫完，不會在多條筆畫之間來回跳針。
fn build_strips(rows: &[(usize, Vec<(f64, f64)>)]) -> Vec<Strip> {
    let mut strips: Vec<Strip> = Vec::new();
    let mut active: Vec<usize> = Vec::new(); // 上一列有接續的條
    for (k, spans) in rows {
        let mut next_active = Vec::new();
        let mut taken: Vec<usize> = Vec::new();
        for &span in spans {
            let found = active.iter().copied().find(|&i| {
                if taken.contains(&i) {
                    return false;
                }
                let (a, b) = strips[i].rows.last().unwrap().1;
                b.min(span.1) - a.max(span.0) > 0.0
            });
            let idx = match found {
                Some(i) => {
                    taken.push(i);
                    i
                }
                None => {
                    strips.push(Strip { rows: Vec::new() });
                    strips.len() - 1
                }
            };
            strips[idx].rows.push((*k, span));
            next_active.push(idx);
        }
        active = next_active;
    }
    strips
}

/// 把（已轉成水平掃描的）多邊形切成各列的區段。
/// (列號, 該列的區段)
type Rows = Vec<(usize, Vec<(f64, f64)>)>;

fn scan_rows(rotated: &Polygon, row_spacing: f64, y_offset: f64) -> Option<(Rows, [f64; 2])> {
    let (min, max) = rotated.bounds()?;
    let mut rows: Rows = Vec::new();
    let mut k = 0usize;
    loop {
        let y = min[1] + y_offset + row_spacing * (k as f64 + 0.5);
        if y >= max[1] {
            break;
        }
        let spans: Vec<_> = spans_at(rotated, y)
            .into_iter()
            .filter(|(a, b)| b - a >= 1.0)
            .collect();
        rows.push((k, spans));
        k += 1;
    }
    Some((rows, min))
}

/// 以某個角度鋪針時，會斷成幾條「一筆畫」（碎片）。碎片越少，剪線、跳針越少，針跡也越順。
pub fn fragment_count(poly: &Polygon, angle_deg: f64, row_spacing: f64) -> usize {
    let rotated = rotate_poly(&poly.normalized(), -angle_deg);
    scan_rows(&rotated, row_spacing, 0.0).map_or(0, |(rows, _)| build_strips(&rows).len())
}

/// 候選角度的個數（0 到 180 度均分）。
const ANGLE_CANDIDATES: usize = 16;

/// 在 16 個候選角度中挑碎片最少的；一樣少時取最接近 `prefer`（度）的。
/// 做法來自「由掃描影像自動產生刺繡」的專利（US7016756 家族）：試多個掃描角度，取碎片最少者。
pub fn best_fill_angle(poly: &Polygon, row_spacing: f64, prefer: f64) -> f64 {
    let gap = |a: f64| {
        let d = (a - prefer).rem_euclid(180.0);
        d.min(180.0 - d)
    };
    (0..ANGLE_CANDIDATES)
        .map(|i| i as f64 * 180.0 / ANGLE_CANDIDATES as f64)
        .map(|a| (fragment_count(poly, a, row_spacing), a))
        .min_by(|x, y| x.0.cmp(&y.0).then(gap(x.1).partial_cmp(&gap(y.1)).unwrap()))
        .map_or(prefer, |(_, a)| a)
}

/// 產生填充針跡。每列的格點錯開三分之一針長；同一條內蛇行往返。
pub fn fill(poly: &Polygon, p: &FillParams) -> Vec<Run> {
    let rotated = rotate_poly(&poly.normalized(), -p.angle_deg);
    let Some((rows, min)) = scan_rows(&rotated, p.row_spacing, p.y_offset) else {
        return Vec::new();
    };

    let mut runs = Vec::new();
    for strip in build_strips(&rows) {
        for (j, (k, (x0, x1))) in strip.rows.iter().enumerate() {
            let y = min[1] + p.y_offset + p.row_spacing * (*k as f64 + 0.5);
            let offset = (k % 3) as f64 * p.max_stitch / 3.0;
            let mut xs = row_points(*x0, *x1, offset, p);
            if j % 2 == 1 {
                xs.reverse();
            }
            runs.push(
                xs.into_iter()
                    .map(|x| rotate([x, y], p.angle_deg))
                    .collect(),
            );
        }
    }
    runs
}
