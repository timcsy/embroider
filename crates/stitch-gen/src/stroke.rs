//! 細長筆畫：判斷一塊形狀該用填充、緞面還是平針，並由形狀取出中線。
//! 做法來自商用數位化的通則：細的線走中線平針、窄的柱狀走緞面、寬的才填充。
//! 中線取法：把形狀點陣化 → 細線化（Zhang-Suen）→ 追成路徑 → 修掉短分叉 → 平滑，
//! 每個點記下該處的半寬（由距離變換得到），緞面靠它決定左右針點的位置。
//! 全部是純函式。單位 0.1mm。

use crate::Run;
use stitch_geom::{spans_at, Polygon};

/// 一塊形狀的繡法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeKind {
    /// 細線：沿中線走平針
    Run,
    /// 窄柱：沿中線走緞面
    Satin,
    /// 其他：鋪填充
    Fill,
}

/// 小於這個「平均寬度」走平針（1mm）。
pub const RUN_MAX_WIDTH: f64 = 10.0;
/// 小於這個「平均寬度」走緞面（約 4mm；再寬的緞面單針太長，容易勾到）。
pub const SATIN_MAX_WIDTH: f64 = 38.0;
/// 長度至少是寬度的幾倍才算「筆畫」，否則是點或塊。
const MIN_ELONGATION: f64 = 4.0;
/// 中線至少是（最寬處）寬度的幾倍，才算筆畫；放射狀的短分叉（花心、圓點）不算。
const MIN_STROKE_LENGTH: f64 = 5.0;
/// 緞面兩條針線之間的距離（0.4mm）。
pub const SATIN_STEP: f64 = 4.0;
/// 點陣化的格子大小（0.2mm）。
const CELL: f64 = 2.0;

fn perimeter(poly: &Polygon) -> f64 {
    poly.rings
        .iter()
        .map(|r| {
            (0..r.len())
                .map(|i| {
                    let (a, b) = (r[i], r[(i + 1) % r.len()]);
                    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt()
                })
                .sum::<f64>()
        })
        .sum()
}

/// 依形狀的平均寬度（2×面積÷周長）與長寬比決定繡法。`poly` 應是單一相連的形狀。
pub fn classify(poly: &Polygon) -> StrokeKind {
    let n = poly.normalized();
    let (area, p) = (n.area(), perimeter(&n));
    if area <= 0.0 || p <= 0.0 {
        return StrokeKind::Fill;
    }
    let t = 2.0 * area / p;
    let length = p / 2.0 - t;
    if length < MIN_ELONGATION * t {
        return StrokeKind::Fill;
    }
    if t < RUN_MAX_WIDTH {
        StrokeKind::Run
    } else if t <= SATIN_MAX_WIDTH {
        StrokeKind::Satin
    } else {
        StrokeKind::Fill
    }
}

/// 判斷形狀是不是真的筆畫：是的話回傳繡法與中線，否則（含「周長很長但中間很胖」的花朵形）回傳 `None`，改用填充。
pub fn plan_stroke(poly: &Polygon) -> Option<(StrokeKind, Vec<Vec<[f64; 3]>>)> {
    let kind = classify(poly);
    if kind == StrokeKind::Fill {
        return None;
    }
    let lines = centerlines(poly);
    let widest = lines
        .iter()
        .flatten()
        .map(|p| 2.0 * p[2])
        .fold(0.0, f64::max);
    let longest = lines
        .iter()
        .map(|l| {
            l.windows(2)
                .map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt())
                .sum::<f64>()
        })
        .fold(0.0, f64::max);
    let limit = match kind {
        StrokeKind::Run => 2.0 * RUN_MAX_WIDTH,
        _ => 1.3 * SATIN_MAX_WIDTH,
    };
    if lines.is_empty() || widest > limit || longest < MIN_STROKE_LENGTH * widest.max(RUN_MAX_WIDTH)
    {
        return None;
    }
    Some((kind, lines))
}

/// 形狀的主軸方向（度，0 到 180，座標系與填充角度相同）；接近圓或方塊時沒有主軸。
/// 長形狀沿長邊鋪針，針比較長、跳針比較少，也比較順著形狀。
pub fn principal_angle(poly: &Polygon) -> Option<f64> {
    let n = poly.normalized();
    let mut pts: Vec<[f64; 2]> = Vec::new();
    for r in &n.rings {
        for i in 0..r.len() {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
            let k = (len / 5.0).ceil().max(1.0) as usize;
            for j in 0..k {
                let t = j as f64 / k as f64;
                pts.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
            }
        }
    }
    if pts.len() < 3 {
        return None;
    }
    let m = pts.len() as f64;
    let (mx, my) = (
        pts.iter().map(|p| p[0]).sum::<f64>() / m,
        pts.iter().map(|p| p[1]).sum::<f64>() / m,
    );
    let (mut cxx, mut cyy, mut cxy) = (0.0, 0.0, 0.0);
    for p in &pts {
        let (dx, dy) = (p[0] - mx, p[1] - my);
        cxx += dx * dx;
        cyy += dy * dy;
        cxy += dx * dy;
    }
    let tr = cxx + cyy;
    let det = cxx * cyy - cxy * cxy;
    let disc = (tr * tr / 4.0 - det).max(0.0).sqrt();
    let (l1, l2) = (tr / 2.0 + disc, tr / 2.0 - disc);
    if l2 <= 0.0 || l1 / l2 < 1.6 {
        return None;
    }
    let deg = (0.5 * (2.0 * cxy).atan2(cxx - cyy)).to_degrees();
    Some(deg.rem_euclid(180.0))
}

// ---- 點陣化與細線化 ----

struct Grid {
    w: usize,
    h: usize,
    ox: f64,
    oy: f64,
    on: Vec<bool>,
}

fn rasterize(poly: &Polygon) -> Option<Grid> {
    let (min, max) = poly.bounds()?;
    let (ox, oy) = (min[0] - 2.0 * CELL, min[1] - 2.0 * CELL);
    let w = ((max[0] - min[0]) / CELL).ceil() as usize + 5;
    let h = ((max[1] - min[1]) / CELL).ceil() as usize + 5;
    let mut on = vec![false; w * h];
    for row in 0..h {
        let y = oy + (row as f64 + 0.5) * CELL;
        for (a, b) in spans_at(poly, y) {
            for col in 0..w {
                let x = ox + (col as f64 + 0.5) * CELL;
                if x >= a && x < b {
                    on[row * w + col] = true;
                }
            }
        }
    }
    Some(Grid { w, h, ox, oy, on })
}

/// 每格到最近空白格的距離（單位：格），用兩趟的近似歐氏距離。
fn distance(g: &Grid) -> Vec<f64> {
    let (w, h) = (g.w, g.h);
    let inf = 1e9;
    let mut d: Vec<f64> = g.on.iter().map(|&o| if o { inf } else { 0.0 }).collect();
    let s2 = std::f64::consts::SQRT_2;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if d[i] == 0.0 {
                continue;
            }
            let mut v = d[i];
            if x > 0 {
                v = v.min(d[i - 1] + 1.0);
            }
            if y > 0 {
                v = v.min(d[i - w] + 1.0);
                if x > 0 {
                    v = v.min(d[i - w - 1] + s2);
                }
                if x + 1 < w {
                    v = v.min(d[i - w + 1] + s2);
                }
            }
            d[i] = v;
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            if d[i] == 0.0 {
                continue;
            }
            let mut v = d[i];
            if x + 1 < w {
                v = v.min(d[i + 1] + 1.0);
            }
            if y + 1 < h {
                v = v.min(d[i + w] + 1.0);
                if x + 1 < w {
                    v = v.min(d[i + w + 1] + s2);
                }
                if x > 0 {
                    v = v.min(d[i + w - 1] + s2);
                }
            }
            d[i] = v;
        }
    }
    d
}

/// 周圍八格，順序為 北、東北、東、東南、南、西南、西、西北。
const RING: [(isize, isize); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

fn at(on: &[bool], w: usize, i: usize, k: usize) -> bool {
    let (dx, dy) = RING[k];
    on[(i as isize + dy * w as isize + dx) as usize]
}

/// Zhang-Suen 細線化。格子外圍有空白邊，所以不會越界。
fn thin(g: &mut Grid) {
    let (w, h) = (g.w, g.h);
    loop {
        let mut changed = false;
        for step in 0..2 {
            let mut kill = Vec::new();
            for y in 1..h - 1 {
                for x in 1..w - 1 {
                    let i = y * w + x;
                    if !g.on[i] {
                        continue;
                    }
                    let n: Vec<bool> = (0..8).map(|k| at(&g.on, w, i, k)).collect();
                    let b = n.iter().filter(|&&v| v).count();
                    let a = (0..8).filter(|&k| !n[k] && n[(k + 1) % 8]).count();
                    // n[0]=北(P2) n[2]=東(P4) n[4]=南(P6) n[6]=西(P8)
                    let cond = if step == 0 {
                        !(n[2] && n[4] && (n[0] || n[6]))
                    } else {
                        !(n[0] && n[6] && (n[2] || n[4]))
                    };
                    if (2..=6).contains(&b) && a == 1 && cond {
                        kill.push(i);
                    }
                }
            }
            if !kill.is_empty() {
                changed = true;
                for i in kill {
                    g.on[i] = false;
                }
            }
        }
        if !changed {
            break;
        }
    }
}

fn degree(on: &[bool], w: usize, i: usize) -> usize {
    (0..8).filter(|&k| at(on, w, i, k)).count()
}

fn neighbors(on: &[bool], w: usize, i: usize) -> Vec<usize> {
    (0..8)
        .filter(|&k| at(on, w, i, k))
        .map(|k| (i as isize + RING[k].1 * w as isize + RING[k].0) as usize)
        .collect()
}

/// 拿掉「拿掉也不會斷」的階梯角落格，讓路徑變成一格寬。
fn deburr(g: &mut Grid) {
    let (w, h) = (g.w, g.h);
    loop {
        let mut changed = false;
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                let i = y * w + x;
                if !g.on[i] {
                    continue;
                }
                let nb = neighbors(&g.on, w, i);
                if nb.len() < 2 {
                    continue;
                }
                // 鄰居彼此要連成一塊，拿掉這格才不會斷
                let adj = |a: usize, b: usize| {
                    let (ax, ay) = ((a % w) as isize, (a / w) as isize);
                    let (bx, by) = ((b % w) as isize, (b / w) as isize);
                    (ax - bx).abs() <= 1 && (ay - by).abs() <= 1
                };
                let mut seen = vec![false; nb.len()];
                let mut stack = vec![0usize];
                seen[0] = true;
                while let Some(c) = stack.pop() {
                    for j in 0..nb.len() {
                        if !seen[j] && adj(nb[c], nb[j]) {
                            seen[j] = true;
                            stack.push(j);
                        }
                    }
                }
                if seen.iter().all(|&s| s) {
                    g.on[i] = false;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
}

struct Branch {
    cells: Vec<usize>,
    /// 起點／終點是不是筆畫的自由端（細線上度數為 1 的格子）
    free_start: bool,
    free_end: bool,
}

fn trace_branches(g: &Grid) -> Vec<Branch> {
    let (w, on) = (g.w, &g.on);
    let deg: Vec<usize> = (0..on.len())
        .map(|i| if on[i] { degree(on, w, i) } else { 0 })
        .collect();
    let mut used = std::collections::HashSet::new();
    let mut out = Vec::new();
    let mark = |used: &mut std::collections::HashSet<(usize, usize)>, a: usize, b: usize| {
        used.insert((a, b));
        used.insert((b, a));
    };
    for n in (0..on.len()).filter(|&i| on[i] && deg[i] != 2) {
        for m in neighbors(on, w, n) {
            if used.contains(&(n, m)) {
                continue;
            }
            mark(&mut used, n, m);
            let mut cells = vec![n, m];
            let (mut prev, mut cur) = (n, m);
            while deg[cur] == 2 {
                let next = neighbors(on, w, cur).into_iter().find(|&c| c != prev);
                let Some(next) = next else { break };
                if used.contains(&(cur, next)) {
                    break;
                }
                mark(&mut used, cur, next);
                cells.push(next);
                prev = cur;
                cur = next;
            }
            let (s, e) = (deg[cells[0]] == 1, deg[*cells.last().unwrap()] == 1);
            out.push(Branch {
                cells,
                free_start: s,
                free_end: e,
            });
        }
    }
    // 沒有端點也沒有分岔的封閉圈
    for start in (0..on.len()).filter(|&i| on[i] && deg[i] == 2) {
        let Some(first) = neighbors(on, w, start)
            .into_iter()
            .find(|&m| !used.contains(&(start, m)))
        else {
            continue;
        };
        mark(&mut used, start, first);
        let mut cells = vec![start, first];
        let (mut prev, mut cur) = (start, first);
        while cur != start {
            let next = neighbors(on, w, cur)
                .into_iter()
                .find(|&c| c != prev && !used.contains(&(cur, c)));
            let Some(next) = next else { break };
            mark(&mut used, cur, next);
            cells.push(next);
            prev = cur;
            cur = next;
        }
        out.push(Branch {
            cells,
            free_start: false,
            free_end: false,
        });
    }
    out
}

/// 去掉接在分岔點上的短分叉（細線化在轉角留下的毛刺），再把只剩兩條的接點接起來。
fn prune_and_join(mut branches: Vec<Branch>, dist: &[f64]) -> Vec<Branch> {
    let junction_of = |b: &Branch| -> Option<usize> {
        match (b.free_start, b.free_end) {
            (true, false) => Some(b.cells[b.cells.len() - 1]),
            (false, true) => Some(b.cells[0]),
            _ => None,
        }
    };
    branches.retain(|b| match junction_of(b) {
        Some(j) => (b.cells.len() as f64) >= 1.5 * dist[j] + 2.0,
        None => true,
    });
    // 接點上只剩兩段的合併成一段
    loop {
        let mut merged = false;
        'search: for a in 0..branches.len() {
            for end_a in [false, true] {
                let pa = if end_a {
                    branches[a].cells[branches[a].cells.len() - 1]
                } else {
                    branches[a].cells[0]
                };
                if (end_a && branches[a].free_end) || (!end_a && branches[a].free_start) {
                    continue;
                }
                let ends: Vec<(usize, bool)> = branches
                    .iter()
                    .enumerate()
                    .flat_map(|(i, b)| {
                        [
                            (i, false, b.cells[0], b.free_start),
                            (i, true, *b.cells.last().unwrap(), b.free_end),
                        ]
                    })
                    .filter(|&(_, _, c, free)| c == pa && !free)
                    .map(|(i, e, _, _)| (i, e))
                    .collect();
                if ends.len() != 2 || ends[0].0 == ends[1].0 {
                    continue;
                }
                let ((i, ei), (j, ej)) = (ends[0], ends[1]);
                let (i, ei, j, ej) = if i < j {
                    (i, ei, j, ej)
                } else {
                    (j, ej, i, ei)
                };
                let mut first = std::mem::replace(
                    &mut branches[i],
                    Branch {
                        cells: Vec::new(),
                        free_start: false,
                        free_end: false,
                    },
                );
                let mut second = branches.remove(j);
                // first 要以接點結尾、second 要以接點開頭
                if !ei {
                    first.cells.reverse();
                    std::mem::swap(&mut first.free_start, &mut first.free_end);
                }
                if ej {
                    second.cells.reverse();
                    std::mem::swap(&mut second.free_start, &mut second.free_end);
                }
                first.cells.extend(second.cells.into_iter().skip(1));
                first.free_end = second.free_end;
                branches[i] = first;
                merged = true;
                break 'search;
            }
        }
        if !merged {
            break;
        }
    }
    branches
}

/// 取出形狀的中線：每條是 `[x, y, 半寬]` 的點序列。形狀應是單一相連的筆畫。
pub fn centerlines(poly: &Polygon) -> Vec<Vec<[f64; 3]>> {
    let Some(mut grid) = rasterize(poly) else {
        return Vec::new();
    };
    let dist = distance(&grid);
    thin(&mut grid);
    deburr(&mut grid);
    let branches = prune_and_join(trace_branches(&grid), &dist);
    let w = grid.w;
    let mut out = Vec::new();
    for b in branches {
        if b.cells.len() < 2 {
            continue;
        }
        let mut pts: Vec<[f64; 3]> = b
            .cells
            .iter()
            .map(|&c| {
                [
                    grid.ox + ((c % w) as f64 + 0.5) * CELL,
                    grid.oy + ((c / w) as f64 + 0.5) * CELL,
                    (dist[c] + 0.5) * CELL,
                ]
            })
            .collect();
        smooth(&mut pts);
        if b.free_start {
            extend(&mut pts, true);
        }
        if b.free_end {
            extend(&mut pts, false);
        }
        out.push(pts);
    }
    out
}

/// 前後各幾點做移動平均；兩端固定。
fn smooth(pts: &mut [[f64; 3]]) {
    const R: usize = 3;
    if pts.len() < 2 * R + 1 {
        return;
    }
    let src = pts.to_vec();
    for i in R..pts.len() - R {
        for k in 0..3 {
            pts[i][k] = (i - R..=i + R).map(|j| src[j][k]).sum::<f64>() / (2 * R + 1) as f64;
        }
    }
}

/// 自由端往外延伸一個半寬，讓緞面蓋到筆畫的盡頭。
fn extend(pts: &mut Vec<[f64; 3]>, at_start: bool) {
    let n = pts.len();
    let (end, back) = if at_start {
        (pts[0], pts[n.min(5) - 1])
    } else {
        (pts[n - 1], pts[n - n.min(5)])
    };
    let (dx, dy) = (end[0] - back[0], end[1] - back[1]);
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-9 {
        return;
    }
    let np = [
        end[0] + dx / len * end[2],
        end[1] + dy / len * end[2],
        end[2],
    ];
    if at_start {
        pts.insert(0, np);
    } else {
        pts.push(np);
    }
}

// ---- 針跡 ----

/// 沿中線等距取樣（含兩端），相鄰取樣點的距離不超過 `step`。
fn resample(line: &[[f64; 3]], step: f64) -> Vec<[f64; 3]> {
    let mut out = vec![line[0]];
    let mut carry = 0.0;
    for w in line.windows(2) {
        let seg = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
        let mut at = step - carry;
        while at <= seg {
            let t = at / seg;
            out.push([
                w[0][0] + (w[1][0] - w[0][0]) * t,
                w[0][1] + (w[1][1] - w[0][1]) * t,
                w[0][2] + (w[1][2] - w[0][2]) * t,
            ]);
            at += step;
        }
        carry = seg - (at - step);
    }
    let last = *line.last().unwrap();
    let tail = out.last().unwrap();
    if (tail[0] - last[0]).abs() > 1e-9 || (tail[1] - last[1]).abs() > 1e-9 {
        out.push(last);
    }
    out
}

/// 單趟沿中線走針，針距不超過 `spacing`。也拿來當緞面的底針。
pub fn center_run(line: &[[f64; 3]], spacing: f64) -> Run {
    resample(line, spacing)
        .into_iter()
        .map(|p| [p[0], p[1]])
        .collect()
}

/// 細線的平針：沿中線「去、回、再去」共三趟，線才夠明顯。
pub fn run_stitch(line: &[[f64; 3]], spacing: f64) -> Vec<Run> {
    let fwd = center_run(line, spacing);
    let mut run = fwd.clone();
    run.extend(fwd.iter().rev().skip(1));
    run.extend(fwd.iter().skip(1));
    vec![run]
}

/// 緞面：沿中線每隔 `SATIN_STEP` 在左右兩側輪流下針，兩側各比筆畫多 `comp`（拉伸補償）。
pub fn satin(line: &[[f64; 3]], comp: f64) -> Vec<Run> {
    let s = resample(line, SATIN_STEP);
    if s.len() < 2 {
        return Vec::new();
    }
    let n = s.len();
    let mut run: Run = Vec::with_capacity(n);
    for (i, p) in s.iter().enumerate() {
        let (a, b) = (s[i.saturating_sub(2)], s[(i + 2).min(n - 1)]);
        let (tx, ty) = (b[0] - a[0], b[1] - a[1]);
        let len = (tx * tx + ty * ty).sqrt().max(1e-9);
        let (nx, ny) = (-ty / len, tx / len);
        let side = if i % 2 == 0 { 1.0 } else { -1.0 };
        let r = p[2] + comp;
        run.push([p[0] + nx * r * side, p[1] + ny * r * side]);
    }
    vec![run]
}
