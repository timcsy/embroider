use stitch_gen::fill::FillParams;
use stitch_gen::stroke::flow_fill;
use stitch_geom::Polygon;

/// 圓弧帶：內外半徑 r0、r1，從 0 度到 `sweep` 度，圓心在原點。
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

fn params() -> FillParams {
    FillParams::default()
}

#[test]
fn 彎曲的帶子針跡順著彎度走() {
    let poly = arc_band(300.0, 400.0, 100.0);
    let runs = flow_fill(&poly, &params()).expect("彎帶應該適用");
    // 針跡方向與「從圓心往外」的方向接近垂直（也就是順著弧走）
    let (mut sum, mut n) = (0.0, 0.0);
    for run in &runs {
        for w in run.windows(2) {
            let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
            let len = (dx * dx + dy * dy).sqrt();
            let (mx, my) = ((w[0][0] + w[1][0]) / 2.0, (w[0][1] + w[1][1]) / 2.0);
            let rl = (mx * mx + my * my).sqrt();
            if len < 5.0 || rl == 0.0 {
                continue;
            }
            sum += ((dx * mx + dy * my) / (len * rl)).abs();
            n += 1.0;
        }
    }
    assert!(n > 50.0, "針數太少：{n}");
    assert!(sum / n < 0.2, "針跡沒有順著弧：平均 |cos| = {}", sum / n);
}

#[test]
fn 針點都在區域內且針距不超過上限() {
    let poly = arc_band(300.0, 400.0, 100.0);
    let loose = poly.offset(1.5);
    let runs = flow_fill(&poly, &params()).unwrap();
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
fn 列數與寬度相符_沒有留白() {
    let poly = arc_band(300.0, 400.0, 100.0);
    let runs = flow_fill(&poly, &params()).unwrap();
    // 寬 10mm、行距 0.4mm → 約 25 列
    assert!((20..=32).contains(&runs.len()), "列數 {}", runs.len());
}

#[test]
fn 直的長條不適用() {
    let bar = Polygon::new(vec![vec![
        [0.0, 0.0],
        [600.0, 0.0],
        [600.0, 100.0],
        [0.0, 100.0],
    ]]);
    assert!(flow_fill(&bar, &params()).is_none());
}

#[test]
fn 彎度大到內側會折疊就不適用() {
    // 寬 10mm、內半徑只有 2mm 的半圓環：內側偏移會自己交叉
    let poly = arc_band(20.0, 120.0, 180.0);
    assert!(flow_fill(&poly, &params()).is_none());
}
