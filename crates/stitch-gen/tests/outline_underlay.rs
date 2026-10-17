use stitch_gen::outline::outline;
use stitch_gen::underlay::underlay;
use stitch_geom::Polygon;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon {
    Polygon::new(vec![vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]])
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

#[test]
fn 輪廓走針沿著邊且閉合() {
    let runs = outline(&rect(0.0, 0.0, 100.0, 60.0), 25.0);
    assert_eq!(runs.len(), 1);
    let r = &runs[0];
    assert_eq!(r.first(), r.last()); // 閉合
    for w in r.windows(2) {
        assert!(dist(w[0], w[1]) <= 25.0 + 1e-9);
    }
    for corner in [[0.0, 0.0], [100.0, 0.0], [100.0, 60.0], [0.0, 60.0]] {
        assert!(r.contains(&corner), "缺少轉角 {corner:?}");
    }
}

#[test]
fn 洞也會各自走一圈() {
    let poly = Polygon::new(vec![
        rect(0.0, 0.0, 100.0, 100.0).rings[0].clone(),
        vec![[30.0, 30.0], [30.0, 70.0], [70.0, 70.0], [70.0, 30.0]],
    ]);
    assert_eq!(outline(&poly, 25.0).len(), 2);
}

#[test]
fn 空多邊形沒有輪廓() {
    assert!(outline(&Polygon::default(), 25.0).is_empty());
}

#[test]
fn 底針內縮在原區域內且比面針稀疏() {
    let poly = rect(0.0, 0.0, 100.0, 100.0);
    let runs = underlay(&poly, 45.0);
    assert!(!runs.is_empty());
    for run in &runs {
        for p in run {
            // 邊緣底針內縮 3，其餘在內縮範圍內
            assert!(p[0] >= 3.0 - 1e-6 && p[0] <= 97.0 + 1e-6, "{p:?}");
            assert!(p[1] >= 3.0 - 1e-6 && p[1] <= 97.0 + 1e-6, "{p:?}");
        }
    }
    // 總針數遠少於 4（0.4mm）行距的面針
    let fill_rows = stitch_gen::fill::fill(&poly, &stitch_gen::fill::FillParams::default());
    let under_points: usize = runs.iter().map(|r| r.len()).sum();
    let fill_points: usize = fill_rows.iter().map(|r| r.len()).sum();
    assert!(
        under_points * 3 < fill_points,
        "底針 {under_points} 面針 {fill_points}"
    );
}

#[test]
fn 太小的區域沒有底針() {
    assert!(underlay(&rect(0.0, 0.0, 5.0, 5.0), 45.0).is_empty());
}
