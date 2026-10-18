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

/// 輪廓填充前簡化輪廓的容許誤差（0.1mm）：看不出差別，但頂點數少很多。
const SIMPLIFY_TOLERANCE: f64 = 1.0;

fn simplify_polygon(poly: &Polygon) -> Polygon {
    Polygon::new(
        poly.rings
            .iter()
            .map(|r| simplify_ring(r, SIMPLIFY_TOLERANCE))
            .filter(|r| r.len() >= 3)
            .collect(),
    )
}

/// Douglas-Peucker 簡化封閉的環（保留起訖頂點之外的端點選擇：以最遠的兩個頂點切成兩段）。
fn simplify_ring(ring: &[[f64; 2]], tol: f64) -> Vec<[f64; 2]> {
    let n = ring.len();
    if n <= 4 {
        return ring.to_vec();
    }
    // 取距離第 0 點最遠的點當第二個錨點，把環切成兩條開放的折線分別簡化
    let far = (1..n)
        .max_by(|&a, &b| {
            let d =
                |i: usize| (ring[i][0] - ring[0][0]).powi(2) + (ring[i][1] - ring[0][1]).powi(2);
            d(a).partial_cmp(&d(b)).unwrap()
        })
        .unwrap();
    let mut first: Vec<[f64; 2]> = ring[..=far].to_vec();
    let mut second: Vec<[f64; 2]> = ring[far..].to_vec();
    second.push(ring[0]);
    let mut out = rdp(&first, tol);
    out.pop();
    out.extend(rdp(&second, tol));
    out.pop();
    first.clear();
    out
}

fn rdp(pts: &[[f64; 2]], tol: f64) -> Vec<[f64; 2]> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt();
    let mut worst = (0usize, 0.0f64);
    for (i, p) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
        let d = if len < 1e-9 {
            ((p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2)).sqrt()
        } else {
            ((p[0] - a[0]) * dy - (p[1] - a[1]) * dx).abs() / len
        };
        if d > worst.1 {
            worst = (i, d);
        }
    }
    if worst.1 <= tol {
        return vec![a, b];
    }
    let mut left = rdp(&pts[..=worst.0], tol);
    let right = rdp(&pts[worst.0..], tol);
    left.pop();
    left.extend(right);
    left
}

/// 輪廓填充最多繞幾圈（0.4mm 行距下約 80mm 深，遠超過繡框）。
const MAX_CONTOUR_ROUNDS: usize = 200;

/// 輪廓填充（contour fill）：沿著區域邊緣一圈一圈往內縮，針跡順著形狀的輪廓走，
/// 圓潤的形狀（花瓣、水滴）看起來比單一方向的填充自然。圈與圈之間以一般針連接，不用跳針。
/// 第一圈離邊半個行距，之後每圈往內一個行距，直到縮不進去為止。
pub fn contour_fill(poly: &Polygon, p: &FillParams) -> Vec<Run> {
    // 管線出來的多邊形頂點很多（像素階梯平滑後的小弧），先簡化再一圈圈偏移，否則每圈都很慢
    let first = simplify_polygon(&poly.normalized()).offset(-p.row_spacing * 0.5);
    let mut current: Vec<Polygon> = first.shapes();
    let mut runs: Vec<Run> = Vec::new();
    let mut last: Option<[f64; 2]> = None;
    // 保險：偏移運算在極小或退化的形狀上可能一直留下碎屑，限制圈數與最小面積，確保一定會結束
    let min_area = p.row_spacing * p.row_spacing;
    let mut rounds = 0;
    while !current.is_empty() && rounds < MAX_CONTOUR_ROUNDS {
        rounds += 1;
        current.retain(|c| c.area() >= min_area);
        let mut next: Vec<Polygon> = Vec::new();
        for shape in &current {
            for mut ring in crate::outline::outline(shape, p.max_stitch) {
                // 讓這一圈從離上一圈終點最近的地方開始，換圈時就不用跳針
                if let Some(at) = last {
                    ring = rotate_closed(ring, at);
                }
                last = ring.last().copied();
                runs.push(ring);
            }
            // 每圈偏移後再簡化：偏移會在轉角留下很多小弧，不簡化的話頂點數一圈比一圈多，越來越慢
            next.extend(
                shape
                    .offset(-p.row_spacing)
                    .shapes()
                    .into_iter()
                    .map(|sh| simplify_polygon(&sh)),
            );
        }
        current = next;
    }
    runs
}

/// 把封閉的針跡（首尾同點）換成從最接近 `at` 的點開始。
fn rotate_closed(run: Run, at: [f64; 2]) -> Run {
    if run.len() < 3 {
        return run;
    }
    let body = &run[..run.len() - 1];
    let (start, _) = body
        .iter()
        .enumerate()
        .map(|(i, q)| (i, (q[0] - at[0]).powi(2) + (q[1] - at[1]).powi(2)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();
    let mut out: Run = body[start..]
        .iter()
        .chain(body[..start].iter())
        .copied()
        .collect();
    out.push(out[0]);
    out
}
