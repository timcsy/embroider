use proptest::prelude::*;
use stitch_core::hoop::{Hoop, HoopShape};
use stitch_core::{validate_plan, ThreadColor};
use stitch_gen::design::{build_plan, ColoredShape, Settings};
use stitch_gen::fit::{fit_polygon, FitResult};
use stitch_geom::Polygon;

fn hoop(shape: HoopShape, w: i32, h: i32) -> Hoop {
    Hoop {
        id: "t".into(),
        name: "測試".into(),
        width: w,
        height: h,
        shape,
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
fn 圓形框的角落比方形框更容易超出() {
    // 安全半徑 (1000-100)/2 - 1 = 449；300×300 的角落距離約 212 → 放得下
    assert_eq!(
        fit_polygon(
            &centered(600.0, 600.0),
            &hoop(HoopShape::Circle, 1000, 1000),
            0.1
        ),
        FitResult::Fits
    );
    // 方形安全範圍同樣是 449，但 800×800 的角落距離 566 > 449 → 圓形放不下，方形放得下
    let big = centered(800.0, 800.0);
    assert_eq!(
        fit_polygon(&big, &hoop(HoopShape::Rect, 1000, 1000), 0.1),
        FitResult::Fits
    );
    match fit_polygon(&big, &hoop(HoopShape::Circle, 1000, 1000), 0.1) {
        FitResult::Scaled(s) => assert!((s - 449.0 / 565.685).abs() < 0.01, "s={s}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn 橢圓框以兩個半軸分別計算() {
    // 橢圓半軸 449 與 249；點 (449, 0) 剛好在邊上，(0, 249) 也是
    let h = hoop(HoopShape::Ellipse, 1000, 600);
    let line = Polygon::new(vec![vec![
        [-440.0, 0.0],
        [440.0, 0.0],
        [440.0, 1.0],
        [-440.0, 1.0],
    ]]);
    assert_eq!(fit_polygon(&line, &h, 0.1), FitResult::Fits);
    match fit_polygon(&centered(0.0 + 400.0, 600.0), &h, 0.1) {
        FitResult::Scaled(s) => assert!(s < 0.85 && s > 0.7, "s={s}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn 縮放後的頂點全部落在橢圓內() {
    let h = hoop(HoopShape::Ellipse, 1000, 600);
    let poly = centered(1200.0, 700.0);
    let FitResult::Scaled(s) = fit_polygon(&poly, &h, 0.1) else {
        panic!("應該需要縮放")
    };
    for p in poly.scale(s).rings.iter().flatten() {
        assert!(h.safe_contains(stitch_core::Point::new(
            p[0].round() as i32,
            p[1].round() as i32
        )));
    }
}

proptest! {
    #[test]
    fn 任意框形與設計下_沒被阻止的計畫一定在安全範圍內(
        shape_i in 0usize..3,
        hw in 300i32..1600, hh in 300i32..2000,
        w in 20.0f64..1500.0, h in 20.0f64..1500.0,
        dx in -200.0f64..200.0, dy in -200.0f64..200.0,
    ) {
        let shape = [HoopShape::Rect, HoopShape::Circle, HoopShape::Ellipse][shape_i];
        let (hw, hh) = if shape == HoopShape::Circle { (hw, hw) } else { (hw, hh) };
        let hoop = hoop(shape, hw, hh);
        let shapes = vec![ColoredShape {
            polygon: centered(w, h).translate(dx, dy),
            color: ThreadColor::new(0, 0, 0, "黑"),
        }];
        let r = build_plan(&shapes, 100.0, &hoop, &Settings::default());
        if r.blocked.is_none() {
            prop_assert!(validate_plan(&r.plan, &hoop).is_ok());
        }
    }
}
