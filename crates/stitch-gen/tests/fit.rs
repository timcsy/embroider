use stitch_core::hoop::{Hoop, HoopShape};
use stitch_gen::fit::{fit_polygon, FitResult};
use stitch_geom::Polygon;

fn hoop(w: i32, h: i32) -> Hoop {
    Hoop {
        id: "t".into(),
        name: "測試".into(),
        width: w,
        height: h,
        shape: HoopShape::Rect,
        margin: 50,
        builtin: false,
    }
}

fn centered(w: f64, h: f64) -> Polygon {
    Polygon::new(vec![vec![
        [-w / 2.0, -h / 2.0],
        [w / 2.0, -h / 2.0],
        [w / 2.0, h / 2.0],
        [-w / 2.0, h / 2.0],
    ]])
}

#[test]
fn 放得下就不縮放() {
    // 框 500×750，安全範圍 400×650
    match fit_polygon(&centered(300.0, 200.0), &hoop(500, 750), 0.1) {
        FitResult::Fits => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn 超出就等比縮小到剛好放進去() {
    // 設計 800×200，安全寬 400 → 約 0.5（再預留 1 單位的取整餘裕）
    match fit_polygon(&centered(800.0, 200.0), &hoop(500, 750), 0.1) {
        FitResult::Scaled(s) => {
            assert!(s < 0.5 && s > 0.49, "s={s}");
            let scaled = centered(800.0, 200.0).scale(s);
            let (min, max) = scaled.bounds().unwrap();
            assert!(max[0] <= 200.0 - 0.99 && min[0] >= -200.0 + 0.99);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn 縮小後低於可接受下限時阻止並給建議() {
    match fit_polygon(&centered(4000.0, 200.0), &hoop(500, 750), 0.5) {
        FitResult::Blocked(m) => {
            assert_eq!(m.code, "too_big_for_hoop");
            assert!(m.hint.is_some());
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn 以高度為限制的情況() {
    // 安全高 650；設計高 1300 → 約 0.5
    match fit_polygon(&centered(100.0, 1300.0), &hoop(500, 750), 0.1) {
        FitResult::Scaled(s) => assert!(s < 0.5 && s > 0.49, "s={s}"),
        other => panic!("{other:?}"),
    }
}
