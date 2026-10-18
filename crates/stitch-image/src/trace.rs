//! 標籤圖 → 多邊形。每個顏色的像素區域以「橫列線段合併成矩形，再做聯集」得到，
//! 座標是像素的格線座標（原點在左上角，y 向下）。

use stitch_geom::Polygon;

/// 3×3 多數決濾波：被周圍多數像素包圍的孤立雜點會被換成周圍的顏色。透明（-1）不參與也不改變。
pub fn majority_filter(labels: &mut [i16], w: usize, h: usize, iterations: usize) {
    for _ in 0..iterations {
        let src = labels.to_vec();
        for y in 0..h {
            for x in 0..w {
                let cur = src[y * w + x];
                if cur < 0 {
                    continue;
                }
                let mut counts: Vec<(i16, u32)> = Vec::new();
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                        if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                            continue;
                        }
                        let l = src[ny as usize * w + nx as usize];
                        if l < 0 {
                            continue;
                        }
                        match counts.iter_mut().find(|c| c.0 == l) {
                            Some(c) => c.1 += 1,
                            None => counts.push((l, 1)),
                        }
                    }
                }
                let own = counts.iter().find(|c| c.0 == cur).map_or(0, |c| c.1);
                if let Some(&(mode, n)) =
                    counts.iter().max_by_key(|c| (c.1, std::cmp::Reverse(c.0)))
                {
                    if mode != cur && n >= 5 && n > own {
                        labels[y * w + x] = mode;
                    }
                }
            }
        }
    }
}

/// 把面積不到 `min_px` 像素的連通色塊，併入周圍最常見的顏色（包含「背景」-1）。
/// 逐一處理由小到大，一個塊併入之後，鄰居的大小也隨之改變。`min_px` 為 0 時不做事。
pub fn remove_small_regions(labels: &mut [i16], w: usize, h: usize, min_px: usize) {
    if min_px == 0 {
        return;
    }
    remove_small_regions_by(labels, w, h, |_| min_px);
}

/// 所有連通色塊（4 連通）：(標籤, 像素編號)。包含標籤 -1 的背景。
pub fn connected_regions(labels: &[i16], w: usize, h: usize) -> Vec<(i16, Vec<usize>)> {
    let mut seen = vec![false; w * h];
    let mut regions: Vec<(i16, Vec<usize>)> = Vec::new();
    for start in 0..w * h {
        if seen[start] {
            continue;
        }
        let label = labels[start];
        let mut stack = vec![start];
        let mut pixels = Vec::new();
        seen[start] = true;
        while let Some(p) = stack.pop() {
            pixels.push(p);
            let (x, y) = (p % w, p / w);
            let mut push = |q: usize| {
                if !seen[q] && labels[q] == label {
                    seen[q] = true;
                    stack.push(q);
                }
            };
            if x > 0 {
                push(p - 1);
            }
            if x + 1 < w {
                push(p + 1);
            }
            if y > 0 {
                push(p - w);
            }
            if y + 1 < h {
                push(p + w);
            }
        }
        regions.push((label, pixels));
    }
    regions
}

/// 物件數上限：圖案（標籤 ≥ 0）的連通色塊超過 `keep` 個時，只留 `score` 最高的 `keep` 個，
/// 其餘併入周圍的顏色（或背景）。零碎的小物件太多，繡出來會亂、剪線與換針也多。
pub fn drop_to_budget(
    labels: &mut [i16],
    w: usize,
    h: usize,
    keep: usize,
    score: impl Fn(&[usize]) -> f64,
) {
    let mut ranked: Vec<(f64, usize, Vec<usize>)> = connected_regions(labels, w, h)
        .into_iter()
        .filter(|(l, _)| *l >= 0)
        .map(|(_, px)| (score(&px), px[0], px))
        .collect();
    if ranked.len() <= keep {
        return;
    }
    ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then(a.1.cmp(&b.1)));
    let mut dropped = vec![false; w * h];
    for (_, _, px) in ranked.iter().skip(keep) {
        for &p in px {
            dropped[p] = true;
        }
    }
    remove_small_regions_by(labels, w, h, |px| {
        if px.iter().all(|&p| dropped[p]) {
            usize::MAX
        } else {
            0
        }
    });
}

/// 同上，但每個色塊的門檻由 `limit` 依它的像素位置決定（例如重要的地方門檻低、不重要的地方門檻高）。
pub fn remove_small_regions_by(
    labels: &mut [i16],
    w: usize,
    h: usize,
    limit: impl Fn(&[usize]) -> usize,
) {
    loop {
        let regions = connected_regions(labels, w, h);
        // 小於門檻的塊，由小到大逐一併入；併入後鄰居變大，下一輪重新計算
        let mut small: Vec<&(i16, Vec<usize>)> = regions
            .iter()
            .filter(|(_, px)| px.len() < limit(px))
            .collect();
        if small.is_empty() {
            return;
        }
        small.sort_by_key(|(l, px)| (px.len(), px[0], *l));
        let mut changed = false;
        for (own, pixels) in small {
            if labels[pixels[0]] != *own {
                continue;
            }
            let mut counts: Vec<(i16, u32)> = Vec::new();
            for &p in pixels {
                let (x, y) = (p % w, p / w);
                let mut add = |q: usize| {
                    let l = labels[q];
                    if l != *own {
                        match counts.iter_mut().find(|c| c.0 == l) {
                            Some(c) => c.1 += 1,
                            None => counts.push((l, 1)),
                        }
                    }
                };
                if x > 0 {
                    add(p - 1);
                }
                if x + 1 < w {
                    add(p + 1);
                }
                if y > 0 {
                    add(p - w);
                }
                if y + 1 < h {
                    add(p + w);
                }
            }
            if let Some(&(target, _)) = counts.iter().max_by_key(|c| (c.1, std::cmp::Reverse(c.0)))
            {
                for &p in pixels {
                    labels[p] = target;
                }
                changed = true;
            }
        }
        if !changed {
            return;
        }
    }
}

/// 某個標籤的所有像素組成的多邊形（含洞、可能有多個分開的形狀）。
pub fn label_polygon(labels: &[i16], w: usize, h: usize, label: i16) -> Polygon {
    // 每列的連續線段
    let mut open: Vec<(usize, usize, usize)> = Vec::new(); // (x0, x1, 起始列) 還在往下延伸的矩形
    let mut rects: Vec<Vec<[f64; 2]>> = Vec::new();
    let close = |x0: usize, x1: usize, y0: usize, y1: usize, rects: &mut Vec<Vec<[f64; 2]>>| {
        rects.push(vec![
            [x0 as f64, y0 as f64],
            [x1 as f64, y0 as f64],
            [x1 as f64, y1 as f64],
            [x0 as f64, y1 as f64],
        ]);
    };
    for y in 0..=h {
        let mut runs: Vec<(usize, usize)> = Vec::new();
        if y < h {
            let mut x = 0;
            while x < w {
                if labels[y * w + x] == label {
                    let start = x;
                    while x < w && labels[y * w + x] == label {
                        x += 1;
                    }
                    runs.push((start, x));
                } else {
                    x += 1;
                }
            }
        }
        // 與上一列完全相同的線段繼續往下延伸，其餘的收尾
        let mut next_open = Vec::new();
        for &(x0, x1, y0) in &open {
            if runs.contains(&(x0, x1)) {
                next_open.push((x0, x1, y0));
            } else {
                close(x0, x1, y0, y, &mut rects);
            }
        }
        for &(x0, x1) in &runs {
            if !next_open.iter().any(|o| o.0 == x0 && o.1 == x1) {
                next_open.push((x0, x1, y));
            }
        }
        open = next_open;
    }
    if rects.is_empty() {
        return Polygon::default();
    }
    Polygon::new(rects).normalized()
}
