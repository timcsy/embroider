use proptest::prelude::*;
use stitch_gen::fill::{fill, FillParams};
use stitch_geom::Polygon;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon {
    Polygon::new(vec![vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]])
}

fn hole(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
    vec![[x0, y0], [x0, y1], [x1, y1], [x1, y0]]
}

fn params0() -> FillParams {
    FillParams {
        angle_deg: 0.0,
        ..FillParams::default()
    }
}

#[test]
fn 預設值與規格一致() {
    let p = FillParams::default();
    assert_eq!(p.row_spacing, 4.0); // 0.4mm
    assert_eq!(p.max_stitch, 30.0); // 3mm
}

#[test]
fn 矩形的_golden_針跡() {
    let runs = fill(&rect(0.0, 0.0, 100.0, 40.0), &params0());
    assert_eq!(runs.len(), 10); // y = 2, 6, ..., 38
                                // 第 0 列：由左到右，格點 0,30,60,90（偏移 0）
    assert_eq!(
        runs[0],
        vec![
            [0.0, 2.0],
            [30.0, 2.0],
            [60.0, 2.0],
            [90.0, 2.0],
            [100.0, 2.0]
        ]
    );
    // 第 1 列：由右到左，格點偏移 10 → 10,40,70
    assert_eq!(
        runs[1],
        vec![
            [100.0, 6.0],
            [70.0, 6.0],
            [40.0, 6.0],
            [10.0, 6.0],
            [0.0, 6.0]
        ]
    );
    // 第 2 列：由左到右，偏移 20 → 20,50,80
    assert_eq!(
        runs[2],
        vec![
            [0.0, 10.0],
            [20.0, 10.0],
            [50.0, 10.0],
            [80.0, 10.0],
            [100.0, 10.0]
        ]
    );
}

#[test]
fn 每一針長度不超過上限() {
    let runs = fill(&rect(0.0, 0.0, 500.0, 40.0), &params0());
    for run in &runs {
        for w in run.windows(2) {
            let d = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
            assert!(d <= 30.0 + 1e-9, "d={d}");
        }
    }
}

#[test]
fn 洞不會被填() {
    let poly = Polygon::new(vec![
        rect(0.0, 0.0, 100.0, 100.0).rings[0].clone(),
        hole(30.0, 30.0, 70.0, 70.0),
    ]);
    let runs = fill(&poly, &params0());
    for run in &runs {
        for p in run {
            let strictly_in_hole = p[0] > 30.0 && p[0] < 70.0 && p[1] > 30.0 && p[1] < 70.0;
            assert!(!strictly_in_hole, "針落在洞裡：{p:?}");
        }
    }
    // 穿過洞的那幾列被切成左右兩段
    let rows_at_50: Vec<_> = runs
        .iter()
        .filter(|r| (r[0][1] - 50.0).abs() < 2.1)
        .collect();
    assert!(rows_at_50.len() >= 2);
}

#[test]
fn 太窄的區段只下兩針() {
    // 寬 8（小於最短針距 5 的兩倍之外的格點會貼近端點）
    let runs = fill(&rect(0.0, 0.0, 8.0, 8.0), &params0());
    assert!(runs.iter().all(|r| r.len() == 2));
}

#[test]
fn 比一針還窄的區段被略過() {
    let runs = fill(&rect(0.0, 0.0, 0.5, 40.0), &params0());
    assert!(runs.is_empty());
}

#[test]
fn 斜角填充的針點仍在多邊形內() {
    let poly = rect(0.0, 0.0, 120.0, 80.0);
    let runs = fill(
        &poly,
        &FillParams {
            angle_deg: 45.0,
            ..FillParams::default()
        },
    );
    assert!(!runs.is_empty());
    for run in &runs {
        for p in run {
            assert!(poly.contains(*p) || on_boundary(p), "{p:?}");
        }
    }
}

fn on_boundary(p: &[f64; 2]) -> bool {
    let e = 1e-6;
    (p[0].abs() < e || (p[0] - 120.0).abs() < e || p[1].abs() < e || (p[1] - 80.0).abs() < e)
        && p[0] > -e
        && p[0] < 120.0 + e
        && p[1] > -e
        && p[1] < 80.0 + e
}

#[test]
fn 相同輸入得到相同輸出() {
    let a = fill(&rect(0.0, 0.0, 100.0, 40.0), &FillParams::default());
    let b = fill(&rect(0.0, 0.0, 100.0, 40.0), &FillParams::default());
    assert_eq!(a, b);
}

proptest! {
    #[test]
    fn 任意矩形與角度下針點在邊界內且每針不超限(
        w in 10.0f64..300.0, h in 10.0f64..300.0, angle in 0.0f64..180.0
    ) {
        let poly = rect(-w / 2.0, -h / 2.0, w / 2.0, h / 2.0);
        let params = FillParams { angle_deg: angle, ..FillParams::default() };
        let runs = fill(&poly, &params);
        let r = (w * w + h * h).sqrt() / 2.0 + 1e-6;
        for run in &runs {
            for p in run {
                prop_assert!(p[0].abs() <= w / 2.0 + 1e-6 && p[1].abs() <= h / 2.0 + 1e-6, "{:?}", p);
                prop_assert!((p[0] * p[0] + p[1] * p[1]).sqrt() <= r);
            }
            for win in run.windows(2) {
                let d = ((win[1][0] - win[0][0]).powi(2) + (win[1][1] - win[0][1]).powi(2)).sqrt();
                prop_assert!(d <= 30.0 + 1e-6);
            }
        }
    }
}

fn h_shape() -> Polygon {
    // H：兩條腿（寬 20）與中間的橫桿，總高 200、總寬 100
    Polygon::new(vec![vec![
        [0.0, 0.0],
        [20.0, 0.0],
        [20.0, 90.0],
        [80.0, 90.0],
        [80.0, 0.0],
        [100.0, 0.0],
        [100.0, 200.0],
        [80.0, 200.0],
        [80.0, 110.0],
        [20.0, 110.0],
        [20.0, 200.0],
        [0.0, 200.0],
    ]])
}

#[test]
fn 多條筆畫分條處理_同一條內相連不亂跳() {
    use stitch_gen::travel::{assemble, ColorBlock, TravelParams};
    let runs = fill(&h_shape(), &params0());
    let plan = assemble(
        &[ColorBlock {
            color: stitch_core::ThreadColor::new(0, 0, 0, "黑"),
            runs,
        }],
        &TravelParams::default(),
    );
    let jumps = plan
        .stitches
        .iter()
        .filter(|s| s.kind == stitch_core::StitchKind::Jump)
        .count();
    // 逐列來回的話會有上百次跳針；分條後只剩條與條之間的幾次
    assert!(jumps <= 6, "跳針 {jumps} 次");
}

#[test]
fn 分條後每個區段仍然只出現一次() {
    // 各列區段的總長度加總應與多邊形面積 / 行距接近
    let poly = h_shape();
    let runs = fill(&poly, &params0());
    let total: f64 = runs
        .iter()
        .map(|r| (r.last().unwrap()[0] - r[0][0]).abs())
        .sum();
    let expected = poly.area() / 4.0;
    assert!(
        (total - expected).abs() / expected < 0.05,
        "total={total} expected={expected}"
    );
}

fn comb() -> Polygon {
    // 底部的橫梁加上三根向上的齒：順著齒的方向（垂直）鋪針，整塊可以一氣呵成；橫著鋪則斷成三條
    Polygon::new(vec![vec![
        [0.0, 0.0],
        [200.0, 0.0],
        [200.0, 200.0],
        [160.0, 200.0],
        [160.0, 40.0],
        [120.0, 40.0],
        [120.0, 200.0],
        [80.0, 200.0],
        [80.0, 40.0],
        [40.0, 40.0],
        [40.0, 200.0],
        [0.0, 200.0],
    ]])
}

#[test]
fn 碎片數_順著齒鋪比橫著鋪少() {
    use stitch_gen::fill::fragment_count;
    let horizontal = fragment_count(&comb(), 0.0, 4.0);
    let vertical = fragment_count(&comb(), 90.0, 4.0);
    assert!(vertical < horizontal, "垂直 {vertical}，水平 {horizontal}");
}

#[test]
fn 挑碎片最少的角度_梳子選垂直() {
    use stitch_gen::fill::best_fill_angle;
    let a = best_fill_angle(&comb(), 4.0, 45.0);
    assert!((a - 90.0).abs() < 12.0, "選了 {a} 度");
}

#[test]
fn 挑角度_碎片一樣多時取最接近偏好的角度() {
    use stitch_gen::fill::best_fill_angle;
    // 方塊各角度的碎片數都是 1：應回到偏好的 45 度
    let a = best_fill_angle(&rect(0.0, 0.0, 100.0, 100.0), 4.0, 45.0);
    assert!((a - 45.0).abs() < 6.0, "選了 {a} 度");
}
