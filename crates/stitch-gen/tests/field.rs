use stitch_gen::field::field_fill;
use stitch_gen::fill::FillParams;
use stitch_geom::Polygon;

fn arc_band(r0: f64, r1: f64, sweep_deg: f64) -> Polygon {
    let n = 48;
    let at = |r: f64, i: usize| {
        let a = (sweep_deg * i as f64 / n as f64).to_radians();
        [r * a.cos(), r * a.sin()]
    };
    let mut ring: Vec<[f64; 2]> = (0..=n).map(|i| at(r1, i)).collect();
    ring.extend((0..=n).rev().map(|i| at(r0, i)));
    Polygon::new(vec![ring])
}

/// 凹凸不平的葉形（非凸）
fn blob() -> Polygon {
    let n = 90;
    let ring = (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            let r = 200.0 * (1.0 + 0.35 * (3.0 * a).sin());
            [r * a.cos() * 1.4, r * a.sin()]
        })
        .collect();
    Polygon::new(vec![ring])
}

fn params() -> FillParams {
    FillParams::default()
}

fn seg_dist(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 {
        0.0
    } else {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0)
    };
    ((p[0] - a[0] - t * dx).powi(2) + (p[1] - a[1] - t * dy).powi(2)).sqrt()
}

#[test]
fn 彎帶的針跡順著弧走() {
    let poly = arc_band(300.0, 400.0, 100.0);
    let runs = field_fill(&poly, &params()).expect("應該能填");
    let (mut sum, mut n) = (0.0, 0.0);
    for run in &runs {
        for w in run.windows(2) {
            let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
            let len = (dx * dx + dy * dy).sqrt();
            let (mx, my) = ((w[0][0] + w[1][0]) / 2.0, (w[0][1] + w[1][1]) / 2.0);
            let rl = (mx * mx + my * my).sqrt();
            if len < 5.0 {
                continue;
            }
            sum += ((dx * mx + dy * my) / (len * rl)).abs();
            n += 1.0;
        }
    }
    assert!(n > 100.0, "針數 {n}");
    assert!(sum / n < 0.25, "沒有順著弧：平均 |cos| = {}", sum / n);
}

#[test]
fn 凹凸形狀針點在區域內且針距不超過上限() {
    let poly = blob();
    let loose = poly.offset(3.0);
    let runs = field_fill(&poly, &params()).expect("應該能填");
    for run in &runs {
        for p in run {
            assert!(loose.contains(*p), "{p:?} 跑出區域");
        }
        for w in run.windows(2) {
            let d = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
            assert!(d <= params().max_stitch + 1e-6, "針距 {d}");
        }
    }
}

#[test]
fn 凹凸形狀幾乎每個點都有針跡經過_沒有大洞() {
    let poly = blob();
    let runs = field_fill(&poly, &params()).unwrap();
    let inner = poly.offset(-6.0);
    let (min, max) = poly.bounds().unwrap();
    let segs: Vec<([f64; 2], [f64; 2])> = runs
        .iter()
        .flat_map(|r| r.windows(2).map(|w| (w[0], w[1])))
        .collect();
    let (mut total, mut far) = (0, 0);
    let mut y = min[1];
    while y < max[1] {
        let mut x = min[0];
        while x < max[0] {
            if inner.contains([x, y]) {
                total += 1;
                let d = segs
                    .iter()
                    .map(|&(a, b)| seg_dist([x, y], a, b))
                    .fold(f64::MAX, f64::min);
                if d > 6.0 {
                    far += 1;
                }
            }
            x += 7.0;
        }
        y += 7.0;
    }
    assert!(total > 300);
    assert!(
        (far as f64) < total as f64 * 0.01,
        "有 {far}/{total} 個點離最近的針跡超過 0.6mm"
    );
}

#[test]
fn 針跡總長與面積相符() {
    let poly = blob();
    let runs = field_fill(&poly, &params()).unwrap();
    let len: f64 = runs
        .iter()
        .flat_map(|r| r.windows(2))
        .map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt())
        .sum();
    let expect = poly.area().abs() / params().row_spacing;
    assert!(
        (0.7..=1.5).contains(&(len / expect)),
        "比例 {}",
        len / expect
    );
}

#[test]
fn 太小的形狀不適用() {
    let tiny = Polygon::new(vec![vec![[0.0, 0.0], [8.0, 0.0], [8.0, 8.0], [0.0, 8.0]]]);
    assert!(field_fill(&tiny, &params()).is_none());
}

#[test]
fn 相同輸入得到相同輸出() {
    let poly = blob();
    assert_eq!(field_fill(&poly, &params()), field_fill(&poly, &params()));
}

#[test]
fn 大形狀不會跑太久() {
    // 6cm × 4cm 的凹凸形
    let poly = blob().scale(1.5);
    let t = std::time::Instant::now();
    let _ = field_fill(&poly, &params());
    assert!(t.elapsed().as_secs_f64() < 10.0, "{:?}", t.elapsed());
}
