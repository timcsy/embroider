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
