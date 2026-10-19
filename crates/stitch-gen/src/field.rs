//! 順著輪廓流動的填充（方向場填充）。
//!
//! 1. 方向場：邊界上的方向取邊緣的切線，內部用擴散平滑地補出來（用「雙倍角」向量，
//!    這樣方向相反也算同一個方向）。
//! 2. 等距流線（Jobard–Lefer）：沿方向場畫線，新線從舊線旁一個行距的地方出發，
//!    離別條線太近就停，所以列與列的間距大致固定、不會擠成一團。
//! 3. 把每條流線每隔一個針長下一針，再把相鄰的線接成順路的次序。
//!
//! 與一般填充相比，針跡會隨形狀彎曲，凹凸、彎曲的區域看起來像手繡的紋路。

use crate::fill::FillParams;
use crate::Run;
use stitch_geom::{spans_at, Polygon};

/// 區域遮罩的格子大小（0.1mm）
const MASK_CELL: f64 = 2.0;
/// 方向場的格子大小
const FIELD_CELL: f64 = 15.0;
/// 方向場擴散的次數
const RELAX_SWEEPS: usize = 300;
/// 畫流線的步長
const STEP: f64 = 2.5;
/// 離別條線小於「行距 × 這個值」就停止
const STOP_RATIO: f64 = 0.8;
/// 新線的起點離舊線至少「行距 × 這個值」
const SEED_RATIO: f64 = 0.97;
/// 短於這個長度的流線不要
const MIN_LINE: f64 = 20.0;
/// 單條流線的最大步數（防止無窮迴圈）
const MAX_STEPS: usize = 40_000;
/// 流線條數的上限
const MAX_LINES: usize = 6_000;
/// 條數不超過這個才做最近端點排序（O(n²)）
const ORDER_LIMIT: usize = 2_500;
/// 總針跡長度與「面積 ÷ 行距」的比例要落在這個範圍，否則視為失敗
const DENSITY_RANGE: (f64, f64) = (0.7, 1.5);

type Pt = [f64; 2];

struct Mask {
    ox: f64,
    oy: f64,
    w: usize,
    h: usize,
    cells: Vec<bool>,
}

impl Mask {
    fn new(poly: &Polygon) -> Option<Mask> {
        let (min, max) = poly.bounds()?;
        let (ox, oy) = (min[0] - 2.0 * MASK_CELL, min[1] - 2.0 * MASK_CELL);
        let w = ((max[0] - ox) / MASK_CELL).ceil() as usize + 3;
        let h = ((max[1] - oy) / MASK_CELL).ceil() as usize + 3;
        let mut cells = vec![false; w * h];
        for j in 0..h {
            let y = oy + (j as f64 + 0.5) * MASK_CELL;
            for (a, b) in spans_at(poly, y) {
                let i0 = (((a - ox) / MASK_CELL) - 0.5).ceil().max(0.0) as usize;
                let i1 = ((((b - ox) / MASK_CELL) - 0.5).floor().max(-1.0) + 1.0) as usize;
                for i in i0..i1.min(w) {
                    cells[j * w + i] = true;
                }
            }
        }
        Some(Mask {
            ox,
            oy,
            w,
            h,
            cells,
        })
    }

    fn at(&self, i: i64, j: i64) -> bool {
        i >= 0
            && j >= 0
            && (i as usize) < self.w
            && (j as usize) < self.h
            && self.cells[j as usize * self.w + i as usize]
    }

    fn inside(&self, p: Pt) -> bool {
        let i = ((p[0] - self.ox) / MASK_CELL).floor() as i64;
        let j = ((p[1] - self.oy) / MASK_CELL).floor() as i64;
        self.at(i, j)
    }
}

/// 方向場：每格存「雙倍角」向量 (cos2θ, sin2θ)。
struct Field {
    ox: f64,
    oy: f64,
    w: usize,
    h: usize,
    v: Vec<[f64; 2]>,
}

impl Field {
    fn new(mask: &Mask, fallback_deg: f64) -> Field {
        let w = (mask.w as f64 * MASK_CELL / FIELD_CELL).ceil() as usize + 1;
        let h = (mask.h as f64 * MASK_CELL / FIELD_CELL).ceil() as usize + 1;
        let mut sum = vec![[0.0f64; 2]; w * h];
        let mut fixed = vec![false; w * h];
        let mut valid = vec![false; w * h];
        let cell_of = |x: f64, y: f64| {
            let i = (((x - mask.ox) / FIELD_CELL).floor().max(0.0) as usize).min(w - 1);
            let j = (((y - mask.oy) / FIELD_CELL).floor().max(0.0) as usize).min(h - 1);
            j * w + i
        };
        for j in 0..mask.h as i64 {
            for i in 0..mask.w as i64 {
                if !mask.at(i, j) {
                    continue;
                }
                let (x, y) = (
                    mask.ox + (i as f64 + 0.5) * MASK_CELL,
                    mask.oy + (j as f64 + 0.5) * MASK_CELL,
                );
                let c = cell_of(x, y);
                valid[c] = true;
                // 半徑 2 格內有幾格在區域外 → 邊界；外側的方向和就是外向法線
                let (mut nx, mut ny, mut out) = (0.0, 0.0, 0);
                for dj in -2i64..=2 {
                    for di in -2i64..=2 {
                        if !mask.at(i + di, j + dj) {
                            nx += di as f64;
                            ny += dj as f64;
                            out += 1;
                        }
                    }
                }
                if out == 0 || (nx == 0.0 && ny == 0.0) {
                    continue;
                }
                let theta = ny.atan2(nx) + std::f64::consts::FRAC_PI_2;
                sum[c][0] += (2.0 * theta).cos();
                sum[c][1] += (2.0 * theta).sin();
                fixed[c] = true;
            }
        }
        let fb = (2.0 * fallback_deg.to_radians()).cos_sin();
        let mut v = vec![[0.0f64; 2]; w * h];
        for c in 0..w * h {
            if fixed[c] {
                let m = (sum[c][0].powi(2) + sum[c][1].powi(2)).sqrt();
                // 兩側方向互相抵消（m≈0）時，退回預設方向
                v[c] = if m > 1e-6 {
                    [sum[c][0] / m, sum[c][1] / m]
                } else {
                    fb
                };
            } else if valid[c] {
                v[c] = [0.0, 0.0];
            }
        }
        for _ in 0..RELAX_SWEEPS {
            for j in 0..h {
                for i in 0..w {
                    let c = j * w + i;
                    if fixed[c] || !valid[c] {
                        continue;
                    }
                    let (mut s, mut n) = ([0.0, 0.0], 0.0);
                    for (di, dj) in [(-1i64, 0i64), (1, 0), (0, -1), (0, 1)] {
                        let (ni, nj) = (i as i64 + di, j as i64 + dj);
                        if ni < 0 || nj < 0 || ni as usize >= w || nj as usize >= h {
                            continue;
                        }
                        let nc = nj as usize * w + ni as usize;
                        if valid[nc] {
                            s[0] += v[nc][0];
                            s[1] += v[nc][1];
                            n += 1.0;
                        }
                    }
                    if n > 0.0 {
                        v[c] = [s[0] / n, s[1] / n];
                    }
                }
            }
        }
        for c in 0..w * h {
            if valid[c] && v[c] == [0.0, 0.0] {
                v[c] = fb;
            }
        }
        Field {
            ox: mask.ox,
            oy: mask.oy,
            w,
            h,
            v,
        }
    }

    /// 某點的方向（單位向量）；雙線性內插雙倍角向量。
    fn dir(&self, p: Pt) -> Pt {
        let fx = ((p[0] - self.ox) / FIELD_CELL - 0.5).clamp(0.0, (self.w - 1) as f64);
        let fy = ((p[1] - self.oy) / FIELD_CELL - 0.5).clamp(0.0, (self.h - 1) as f64);
        let (i0, j0) = (fx.floor() as usize, fy.floor() as usize);
        let (i1, j1) = ((i0 + 1).min(self.w - 1), (j0 + 1).min(self.h - 1));
        let (tx, ty) = (fx - i0 as f64, fy - j0 as f64);
        let g = |i: usize, j: usize| self.v[j * self.w + i];
        let mix = |k: usize| {
            g(i0, j0)[k] * (1.0 - tx) * (1.0 - ty)
                + g(i1, j0)[k] * tx * (1.0 - ty)
                + g(i0, j1)[k] * (1.0 - tx) * ty
                + g(i1, j1)[k] * tx * ty
        };
        let s = [mix(0), mix(1)];
        let theta = 0.5 * s[1].atan2(s[0]);
        [theta.cos(), theta.sin()]
    }
}

trait CosSin {
    fn cos_sin(self) -> [f64; 2];
}
impl CosSin for f64 {
    fn cos_sin(self) -> [f64; 2] {
        [self.cos(), self.sin()]
    }
}

/// 已畫出的流線取樣點，用格狀雜湊找「附近有沒有別的線」。
struct Samples {
    cell: f64,
    ox: f64,
    oy: f64,
    w: usize,
    h: usize,
    grid: Vec<Vec<(Pt, u32, i32)>>,
}

impl Samples {
    fn new(mask: &Mask, cell: f64) -> Samples {
        let w = (mask.w as f64 * MASK_CELL / cell).ceil() as usize + 2;
        let h = (mask.h as f64 * MASK_CELL / cell).ceil() as usize + 2;
        Samples {
            cell,
            ox: mask.ox,
            oy: mask.oy,
            w,
            h,
            grid: vec![Vec::new(); w * h],
        }
    }

    fn ij(&self, p: Pt) -> (i64, i64) {
        (
            ((p[0] - self.ox) / self.cell).floor() as i64,
            ((p[1] - self.oy) / self.cell).floor() as i64,
        )
    }

    fn add(&mut self, p: Pt, line: u32, idx: i32) {
        let (i, j) = self.ij(p);
        if i >= 0 && j >= 0 && (i as usize) < self.w && (j as usize) < self.h {
            self.grid[j as usize * self.w + i as usize].push((p, line, idx));
        }
    }

    /// 距離 `dist`（不超過一格）內有沒有別的線的取樣點；同一條線上索引差在 `skip` 內的不算。
    fn near(&self, p: Pt, dist: f64, line: u32, idx: i32, skip: i32) -> bool {
        let (ci, cj) = self.ij(p);
        for j in cj - 1..=cj + 1 {
            for i in ci - 1..=ci + 1 {
                if i < 0 || j < 0 || i as usize >= self.w || j as usize >= self.h {
                    continue;
                }
                for &(q, l, k) in &self.grid[j as usize * self.w + i as usize] {
                    if l == line && (k - idx).abs() <= skip {
                        continue;
                    }
                    if (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) < dist * dist {
                        return true;
                    }
                }
            }
        }
        false
    }
}

struct Tracer<'a> {
    mask: &'a Mask,
    field: &'a Field,
    stop: f64,
    skip: i32,
}

impl Tracer<'_> {
    /// 從 `seed` 往 `sign` 方向走，直到離開區域或太靠近別的線。回傳走過的點（不含種子）。
    fn walk(&self, seed: Pt, sign: f64, line: u32, samples: &mut Samples) -> Vec<Pt> {
        let mut out = Vec::new();
        let mut p = seed;
        let mut prev = {
            let d = self.field.dir(seed);
            [d[0] * sign, d[1] * sign]
        };
        for n in 1..=MAX_STEPS as i32 {
            // 二階（中點）積分；方向的正負跟著上一步，才不會突然掉頭
            let d1 = self.aligned(p, prev);
            let mid = [p[0] + d1[0] * STEP / 2.0, p[1] + d1[1] * STEP / 2.0];
            let d2 = self.aligned(mid, d1);
            let next = [p[0] + d2[0] * STEP, p[1] + d2[1] * STEP];
            let idx = n * sign as i32;
            if !self.mask.inside(next) || samples.near(next, self.stop, line, idx, self.skip) {
                break;
            }
            samples.add(next, line, idx);
            out.push(next);
            p = next;
            prev = d2;
        }
        out
    }

    fn aligned(&self, p: Pt, prev: Pt) -> Pt {
        let d = self.field.dir(p);
        if d[0] * prev[0] + d[1] * prev[1] < 0.0 {
            [-d[0], -d[1]]
        } else {
            d
        }
    }
}

fn dist(a: Pt, b: Pt) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn path_len(s: &[Pt]) -> f64 {
    s.windows(2).map(|w| dist(w[0], w[1])).sum()
}

/// 方向場填充。形狀太小、跑出的針跡密度不對時回傳 `None`，由呼叫端改用一般填充。
pub fn field_fill(poly: &Polygon, p: &FillParams) -> Option<Vec<Run>> {
    let poly = poly.normalized();
    let area = poly.area().abs();
    if area < 100.0 {
        return None;
    }
    let mask = Mask::new(&poly)?;
    let field = Field::new(&mask, p.angle_deg);
    let sep = p.row_spacing;
    let mut samples = Samples::new(&mask, sep);
    let tracer = Tracer {
        mask: &mask,
        field: &field,
        stop: sep * STOP_RATIO,
        skip: (2.0 * sep / STEP).ceil() as i32 + 2,
    };

    // 第一個種子：中間那一列最長區段的中點
    let (min, max) = poly.bounds()?;
    let mid_y = (min[1] + max[1]) / 2.0;
    let seed0 = (0..=40)
        .flat_map(|k| {
            let y = mid_y
                + (k as f64 / 2.0).ceil() * if k % 2 == 0 { 1.0 } else { -1.0 } * MASK_CELL * 3.0;
            spans_at(&poly, y)
                .into_iter()
                .map(move |(a, b)| (b - a, [(a + b) / 2.0, y]))
        })
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
        .map(|(_, s)| s)?;

    let mut lines: Vec<Vec<Pt>> = Vec::new();
    let mut pending: Vec<Pt> = vec![seed0];
    let mut cursor = 0usize;
    loop {
        // 取下一個種子：先用待處理的，沒有了就從已畫的線旁邊長出新的
        let seed = if let Some(s) = pending.pop() {
            s
        } else if cursor < lines.len() {
            let line = &lines[cursor];
            cursor += 1;
            for k in (0..line.len()).step_by(2) {
                let a = line[k.saturating_sub(1)];
                let b = line[(k + 1).min(line.len() - 1)];
                let l = dist(a, b).max(1e-9);
                let n = [-(b[1] - a[1]) / l, (b[0] - a[0]) / l];
                for s in [1.0, -1.0] {
                    let c = [line[k][0] + n[0] * sep * s, line[k][1] + n[1] * sep * s];
                    pending.push(c);
                }
            }
            pending.reverse();
            continue;
        } else {
            break;
        };
        if lines.len() >= MAX_LINES {
            return None;
        }
        if !mask.inside(seed) || samples.near(seed, sep * SEED_RATIO, u32::MAX, 0, 0) {
            continue;
        }
        let id = lines.len() as u32;
        samples.add(seed, id, 0);
        let fwd = tracer.walk(seed, 1.0, id, &mut samples);
        let back = tracer.walk(seed, -1.0, id, &mut samples);
        let mut line: Vec<Pt> = back.into_iter().rev().collect();
        line.push(seed);
        line.extend(fwd);
        if path_len(&line) >= MIN_LINE {
            lines.push(line);
        }
    }
    if lines.is_empty() {
        return None;
    }

    let order = order_lines(&lines);
    let mut runs: Vec<Run> = Vec::new();
    let mut total = 0.0;
    for (n, (i, reversed)) in order.into_iter().enumerate() {
        let mut line = lines[i].clone();
        if reversed {
            line.reverse();
        }
        total += path_len(&line);
        let stitched = crate::stroke::walk_polyline(&line, p.max_stitch, (n % 3) as f64 / 3.0);
        if stitched.len() >= 2 {
            runs.push(stitched);
        }
    }
    let expected = area / sep;
    let ratio = total / expected;
    if ratio < DENSITY_RANGE.0 || ratio > DENSITY_RANGE.1 {
        return None;
    }
    Some(runs)
}

/// 把流線排成順路的次序：每次接最近的端點，必要時把那條線反過來走。
fn order_lines(lines: &[Vec<Pt>]) -> Vec<(usize, bool)> {
    if lines.len() > ORDER_LIMIT {
        return (0..lines.len()).map(|i| (i, i % 2 == 1)).collect();
    }
    let mut used = vec![false; lines.len()];
    let mut out = Vec::with_capacity(lines.len());
    used[0] = true;
    out.push((0, false));
    let mut end = *lines[0].last().unwrap();
    for _ in 1..lines.len() {
        let mut best: Option<(f64, usize, bool)> = None;
        for (i, l) in lines.iter().enumerate() {
            if used[i] {
                continue;
            }
            let (ds, de) = (dist(end, l[0]), dist(end, *l.last().unwrap()));
            let cand = if ds <= de {
                (ds, i, false)
            } else {
                (de, i, true)
            };
            if best.is_none_or(|b| cand.0 < b.0) {
                best = Some(cand);
            }
        }
        let (_, i, rev) = best.unwrap();
        used[i] = true;
        out.push((i, rev));
        end = if rev {
            lines[i][0]
        } else {
            *lines[i].last().unwrap()
        };
    }
    out
}
