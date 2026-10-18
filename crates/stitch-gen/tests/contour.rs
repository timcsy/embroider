use stitch_gen::fill::{contour_fill, FillParams};
use stitch_geom::Polygon;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon {
    Polygon::new(vec![vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]])
}

fn dist_to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 {
        0.0
    } else {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0)
    };
    ((p[0] - a[0] - t * dx).powi(2) + (p[1] - a[1] - t * dy).powi(2)).sqrt()
}

fn params() -> FillParams {
    FillParams::default()
}

#[test]
fn 輪廓填充的針點都在區域內且針距不超過上限() {
    let poly = rect(0.0, 0.0, 200.0, 200.0);
    let runs = contour_fill(&poly, &params());
    assert!(runs.len() > 10, "圈數太少：{}", runs.len());
    for run in &runs {
        for p in run {
            assert!(
                poly.contains(*p) || p[0] >= 0.0 && p[0] <= 200.0 && p[1] >= 0.0 && p[1] <= 200.0,
                "{p:?}"
            );
        }
        for w in run.windows(2) {
            let d = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
            assert!(d <= params().max_stitch + 1e-6, "針距 {d}");
        }
    }
}

#[test]
fn 輪廓填充蓋滿整塊區域_沒有露出底布的空隙() {
    let poly = rect(0.0, 0.0, 200.0, 120.0);
    let runs = contour_fill(&poly, &params());
    let segs: Vec<([f64; 2], [f64; 2])> = runs
        .iter()
        .flat_map(|r| r.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>())
        .collect();
    let mut y = 3.0;
    while y < 120.0 {
        let mut x = 3.0;
        while x < 200.0 {
            let d = segs
                .iter()
                .map(|s| dist_to_segment([x, y], s.0, s.1))
                .fold(f64::MAX, f64::min);
            assert!(
                d <= params().row_spacing,
                "({x},{y}) 離最近的針線 {d}，有空隙"
            );
            x += 7.0;
        }
        y += 7.0;
    }
}

#[test]
fn 輪廓填充一圈接著一圈_換圈時不用跳針() {
    let runs = contour_fill(&rect(0.0, 0.0, 200.0, 200.0), &params());
    for w in runs.windows(2) {
        let (end, start) = (*w[0].last().unwrap(), w[1][0]);
        let d = ((start[0] - end[0]).powi(2) + (start[1] - end[1]).powi(2)).sqrt();
        assert!(
            d <= 3.0 * params().row_spacing + params().max_stitch,
            "換圈距離 {d}"
        );
    }
}

#[test]
fn 輪廓填充避開洞() {
    let poly = Polygon::new(vec![
        vec![[0.0, 0.0], [200.0, 0.0], [200.0, 200.0], [0.0, 200.0]],
        vec![[60.0, 60.0], [60.0, 140.0], [140.0, 140.0], [140.0, 60.0]],
    ]);
    let runs = contour_fill(&poly, &params());
    for run in &runs {
        for p in run {
            let in_hole = p[0] > 61.0 && p[0] < 139.0 && p[1] > 61.0 && p[1] < 139.0;
            assert!(!in_hole, "針落在洞裡 {p:?}");
        }
    }
}

#[test]
fn 太小而縮不進去的區域沒有輪廓填充() {
    assert!(contour_fill(&rect(0.0, 0.0, 1.0, 1.0), &params()).len() <= 1);
}

#[test]
fn 頂點很多的多邊形先簡化_結果仍蓋滿且在區域內() {
    // 4000 個頂點的圓：模擬管線出來的階梯平滑多邊形
    let ring: Vec<[f64; 2]> = (0..4000)
        .map(|i| {
            let a = i as f64 / 4000.0 * std::f64::consts::TAU;
            [300.0 * a.cos(), 300.0 * a.sin()]
        })
        .collect();
    let poly = Polygon::new(vec![ring]);
    let runs = contour_fill(&poly, &params());
    assert!(runs.len() > 50, "{}", runs.len());
    for run in &runs {
        for p in run {
            assert!((p[0] * p[0] + p[1] * p[1]).sqrt() <= 300.5, "{p:?}");
        }
    }
}

#[test]
fn 邊緣鋸齒的大區域也在幾秒內完成_偏移不會讓頂點越來越多() {
    // 管線出來的區域邊緣是像素階梯平滑後的鋸齒；鋸齒的凹角每偏移一圈就多一堆小弧，
    // 沒有每圈簡化的話頂點數暴增，要跑幾分鐘。
    let ring: Vec<[f64; 2]> = (0..1200)
        .map(|i| {
            let a = i as f64 / 1200.0 * std::f64::consts::TAU;
            let r = 350.0 + if i % 2 == 0 { 3.0 } else { -3.0 } + 6.0 * (a * 7.0).sin();
            [r * a.cos(), r * a.sin()]
        })
        .collect();
    let start = std::time::Instant::now();
    let runs = contour_fill(&Polygon::new(vec![ring]), &params());
    let took = start.elapsed();
    assert!(runs.len() > 50, "{}", runs.len());
    assert!(took.as_secs_f64() < 10.0, "太慢：{took:?}");
}
