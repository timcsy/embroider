use crate::Run;
use stitch_core::{Stitch, StitchPlan, ThreadColor};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TravelParams {
    /// 區段之間超過此距離就用跳針（預設 30 = 3mm）
    pub jump_threshold: f64,
    /// 超過此距離跳針前先剪線（預設 80 = 8mm）
    pub trim_threshold: f64,
    /// 固定針的長度（預設 4 = 0.4mm）
    pub lock_len: f64,
}

impl Default for TravelParams {
    fn default() -> Self {
        Self {
            jump_threshold: 30.0,
            trim_threshold: 80.0,
            lock_len: 4.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorBlock {
    pub color: ThreadColor,
    pub runs: Vec<Run>,
}

/// 相同顏色（RGB）的區塊合併，依首次出現的順序排列，以減少換色。
pub fn group_by_color(blocks: Vec<ColorBlock>) -> Vec<ColorBlock> {
    let mut out: Vec<ColorBlock> = Vec::new();
    for b in blocks {
        match out
            .iter_mut()
            .find(|o| (o.color.r, o.color.g, o.color.b) == (b.color.r, b.color.g, b.color.b))
        {
            Some(existing) => existing.runs.extend(b.runs),
            None => out.push(b),
        }
    }
    out
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

fn round(p: [f64; 2]) -> (i32, i32) {
    (p[0].round() as i32, p[1].round() as i32)
}

struct Sewer {
    out: Vec<Stitch>,
    cur: [f64; 2],
    params: TravelParams,
}

impl Sewer {
    fn stitch(&mut self, p: [f64; 2]) {
        let (x, y) = round(p);
        // 原地不動的針沒有意義（有些讀檔程式會直接丟掉），一律略過
        let dup = matches!(self.out.last(), Some(s) if s.x == x && s.y == y);
        if !dup {
            self.out.push(Stitch::stitch(x, y));
        }
        self.cur = p;
    }

    /// 固定針的方向與可用長度：朝向第一個距離夠遠（至少半個固定針長）的點；
    /// 都不夠遠就朝向最遠的那個點。這樣開頭或結尾有極短的線段時，固定針也不會退化成原地。
    fn direction<'a>(
        &self,
        from: [f64; 2],
        others: impl Iterator<Item = &'a [f64; 2]>,
    ) -> Option<([f64; 2], f64)> {
        let enough = self.params.lock_len * 0.5;
        let mut best: Option<([f64; 2], f64)> = None;
        for &o in others {
            let d = dist(from, o);
            if d <= 1e-9 {
                continue;
            }
            if best.is_none_or(|(_, bd)| d > bd) {
                best = Some((o, d));
            }
            if d >= enough {
                best = Some((o, d));
                break;
            }
        }
        best.map(|(o, d)| ([(o[0] - from[0]) / d, (o[1] - from[1]) / d], d))
    }

    fn lock_in(&mut self, run: &Run) {
        let p = run[0];
        if let Some((u, d)) = self.direction(p, run[1..].iter()) {
            let l = self.params.lock_len.min(d);
            self.stitch([p[0] + u[0] * l, p[1] + u[1] * l]);
            self.stitch(p);
        }
    }

    fn lock_out(&mut self, run: &Run) {
        let p = *run.last().unwrap();
        if let Some((u, d)) = self.direction(p, run[..run.len() - 1].iter().rev()) {
            let l = self.params.lock_len.min(d);
            self.stitch([p[0] + u[0] * l, p[1] + u[1] * l]);
            self.stitch(p);
        }
    }

    fn jump(&mut self, p: [f64; 2]) {
        let (x, y) = round(p);
        self.out.push(Stitch::jump(x, y));
        self.cur = p;
    }
}

/// 把各顏色的區段組成最終針跡：必要時跳針、剪線，區段頭尾加固定針，換色前後收尾。
pub fn assemble(blocks: &[ColorBlock], params: &TravelParams) -> StitchPlan {
    let mut s = Sewer {
        out: Vec::new(),
        cur: [0.0, 0.0],
        params: *params,
    };
    let mut colors: Vec<ThreadColor> = Vec::new();
    let mut last_run: Option<&Run> = None;

    for block in blocks {
        let runs: Vec<&Run> = block.runs.iter().filter(|r| r.len() >= 2).collect();
        if runs.is_empty() {
            continue;
        }
        if !colors.is_empty() {
            if let Some(prev) = last_run.take() {
                s.lock_out(prev);
            }
            let (x, y) = round(s.cur);
            s.out.push(Stitch::color_change(x, y));
        }
        colors.push(block.color.clone());

        let mut first_in_block = true;
        for run in runs {
            let start = run[0];
            let connect_by_stitch = !first_in_block
                && last_run.is_some()
                && dist(s.cur, start) <= params.jump_threshold;
            if connect_by_stitch {
                s.stitch(start);
            } else {
                if let Some(prev) = last_run {
                    s.lock_out(prev);
                    if dist(s.cur, start) > params.trim_threshold {
                        let (x, y) = round(s.cur);
                        s.out.push(Stitch::trim(x, y));
                    }
                }
                if s.cur != start {
                    s.jump(start);
                }
                s.lock_in(run);
            }
            for &p in &run[1..] {
                s.stitch(p);
            }
            first_in_block = false;
            last_run = Some(run);
        }
    }

    if let Some(prev) = last_run {
        s.lock_out(prev);
    }
    let (x, y) = round(s.cur);
    s.out.push(Stitch::end(x, y));
    StitchPlan {
        stitches: s.out,
        colors,
    }
}
