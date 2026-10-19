use stitch_core::hoop::{builtin_hoops, Hoop};
use stitch_core::{validate_plan, StitchKind, ThreadColor};
use stitch_gen::design::{build_plan, ColoredShape, Settings};
use stitch_geom::Polygon;

fn hoop(id: &str) -> Hoop {
    builtin_hoops().into_iter().find(|h| h.id == id).unwrap()
}

fn centered_rect(w: f64, h: f64) -> Polygon {
    Polygon::new(vec![vec![
        [-w / 2.0, -h / 2.0],
        [w / 2.0, -h / 2.0],
        [w / 2.0, h / 2.0],
        [-w / 2.0, h / 2.0],
    ]])
}

fn black() -> ThreadColor {
    ThreadColor::new(0, 0, 0, "黑")
}

fn one(poly: Polygon) -> Vec<ColoredShape> {
    vec![ColoredShape {
        polygon: poly,
        color: black(),
    }]
}

#[test]
fn 放得下的設計產生合法的計畫() {
    let r = build_plan(
        &one(centered_rect(200.0, 100.0)),
        100.0,
        &hoop("b70-medium"),
        &Settings::default(),
    );
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert_eq!(r.fitted_scale, 1.0);
    assert!(r.adjustments.is_empty());
    assert!(r.plan.stitch_count() > 200);
    assert_eq!(r.plan.colors, vec![black()]);
    validate_plan(&r.plan, &hoop("b70-medium")).unwrap();
}

#[test]
fn 太小的設計會自動放大並說明() {
    // 字級 20（2mm）遠低於最小字高 40
    let r = build_plan(
        &one(centered_rect(100.0, 30.0)),
        20.0,
        &hoop("b70-medium"),
        &Settings::default(),
    );
    assert!(r.blocked.is_none());
    assert!(
        r.adjustments.iter().any(|m| m.code == "enlarged"),
        "{:?}",
        r.adjustments
    );
    // 說明裡的「公分」數字要正確：字級 20（2mm）乘上放大倍率，單位 0.1mm → 除以 100 才是公分
    let m = r.adjustments.iter().find(|m| m.code == "enlarged").unwrap();
    let expected_cm = format!("{:.1}", 20.0 * r.enlarge_scale / 100.0);
    assert!(
        m.text.contains(&expected_cm),
        "說明「{}」應含 {expected_cm}",
        m.text
    );
    let (min_x, max_x) = r
        .plan
        .stitches
        .iter()
        .fold((i32::MAX, i32::MIN), |a, s| (a.0.min(s.x), a.1.max(s.x)));
    assert!(max_x - min_x > 150, "寬度 {}", max_x - min_x);
}

#[test]
fn 太大的設計會縮小到放進框並說明() {
    // 小框安全寬 400；設計寬 800
    let r = build_plan(
        &one(centered_rect(800.0, 100.0)),
        100.0,
        &hoop("b70-small"),
        &Settings::default(),
    );
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert!(
        r.fitted_scale < 0.5 && r.fitted_scale > 0.4,
        "{}",
        r.fitted_scale
    );
    assert!(r.adjustments.iter().any(|m| m.code == "shrunk"));
    validate_plan(&r.plan, &hoop("b70-small")).unwrap();
}

#[test]
fn 縮小後會太細的設計被阻止() {
    // 細長條要縮到很小才放得進去，筆畫會細於下限
    let r = build_plan(
        &one(centered_rect(4000.0, 30.0)),
        100.0,
        &hoop("b70-small"),
        &Settings::default(),
    );
    let m = r.blocked.expect("應該被阻止");
    assert_eq!(m.code, "too_big_for_hoop");
    assert!(r.plan.stitches.is_empty());
}

#[test]
fn 多個顏色依序換色() {
    let shapes = vec![
        ColoredShape {
            polygon: centered_rect(100.0, 60.0).translate(-80.0, 0.0),
            color: black(),
        },
        ColoredShape {
            polygon: centered_rect(100.0, 60.0).translate(80.0, 0.0),
            color: ThreadColor::new(200, 30, 30, "正紅"),
        },
    ];
    let r = build_plan(&shapes, 100.0, &hoop("b70-medium"), &Settings::default());
    assert!(r.blocked.is_none());
    assert_eq!(r.plan.color_change_count(), 1);
    assert_eq!(r.plan.colors.len(), 2);
    validate_plan(&r.plan, &hoop("b70-medium")).unwrap();
}

#[test]
fn 同色的分開形狀不會多換色() {
    let shapes = vec![
        ColoredShape {
            polygon: centered_rect(60.0, 60.0).translate(-100.0, 0.0),
            color: black(),
        },
        ColoredShape {
            polygon: centered_rect(60.0, 60.0).translate(100.0, 0.0),
            color: black(),
        },
    ];
    let r = build_plan(&shapes, 100.0, &hoop("b70-medium"), &Settings::default());
    assert_eq!(r.plan.color_change_count(), 0);
}

#[test]
fn 兩個分開的形狀之間用跳針連接() {
    let shapes = one(Polygon::new(vec![
        centered_rect(60.0, 60.0).translate(-150.0, 0.0).rings[0].clone(),
        centered_rect(60.0, 60.0).translate(150.0, 0.0).rings[0].clone(),
    ]));
    let r = build_plan(&shapes, 100.0, &hoop("b70-medium"), &Settings::default());
    let jumps = r
        .plan
        .stitches
        .iter()
        .filter(|s| s.kind == StitchKind::Jump)
        .count();
    assert!(jumps >= 1);
}

#[test]
fn 空輸入被阻止() {
    let r = build_plan(&[], 100.0, &hoop("b70-medium"), &Settings::default());
    assert_eq!(r.blocked.unwrap().code, "empty_design");
}

#[test]
fn 相同輸入得到相同輸出() {
    let a = build_plan(
        &one(centered_rect(200.0, 100.0)),
        100.0,
        &hoop("b70-medium"),
        &Settings::default(),
    );
    let b = build_plan(
        &one(centered_rect(200.0, 100.0)),
        100.0,
        &hoop("b70-medium"),
        &Settings::default(),
    );
    assert_eq!(a.plan, b.plan);
}

#[test]
fn 圓形框也能適配() {
    let mut round = hoop("b70-medium");
    round.shape = stitch_core::hoop::HoopShape::Circle;
    round.width = 1000;
    round.height = 1000;
    let r = build_plan(
        &one(centered_rect(900.0, 300.0)),
        100.0,
        &round,
        &Settings::default(),
    );
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    validate_plan(&r.plan, &round).unwrap();
}

#[test]
fn 兩色交錯混色_兩條線都繡滿同一塊區域() {
    use stitch_core::StitchKind;
    use stitch_gen::design::build_plan_with_blends;
    let red = ThreadColor::new(200, 30, 40, "紅");
    let blue = ThreadColor::new(40, 80, 180, "藍");
    let shape = ColoredShape {
        polygon: centered_rect(400.0, 400.0),
        color: red.clone(),
    };
    let r = build_plan_with_blends(
        &[shape],
        &[Some([red.clone(), blue.clone()])],
        400.0,
        &hoop("b70-large"),
        &Settings::default(),
    );
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert_eq!(r.plan.colors.len(), 2, "{:?}", r.plan.colors);
    // 逐色統計針點的範圍與針數
    let mut color = 0usize;
    let mut stats = [(i32::MAX, i32::MIN, i32::MAX, i32::MIN, 0usize); 2];
    for s in &r.plan.stitches {
        match s.kind {
            StitchKind::ColorChange => color += 1,
            StitchKind::Stitch => {
                let t = &mut stats[color];
                *t = (
                    t.0.min(s.x),
                    t.1.max(s.x),
                    t.2.min(s.y),
                    t.3.max(s.y),
                    t.4 + 1,
                );
            }
            _ => {}
        }
    }
    let (a, b) = (stats[0], stats[1]);
    assert!(a.4 > 100 && b.4 > 100, "兩色針數 {} {}", a.4, b.4);
    // 兩色蓋住同樣的範圍（誤差在一列行距內）
    assert!(
        (a.0 - b.0).abs() < 40 && (a.1 - b.1).abs() < 40,
        "{a:?} {b:?}"
    );
    assert!(
        (a.2 - b.2).abs() < 40 && (a.3 - b.3).abs() < 40,
        "{a:?} {b:?}"
    );
    // 兩色的針數相近（各佔一半的列）
    let ratio = a.4 as f64 / b.4 as f64;
    assert!((0.5..2.0).contains(&ratio), "針數比 {ratio}");
}

#[test]
fn 沒有混色時和原本的結果完全一樣() {
    use stitch_gen::design::build_plan_with_blends;
    let shapes = one(centered_rect(200.0, 100.0));
    let h = hoop("b70-medium");
    let s = Settings::default();
    let plain = build_plan(&shapes, 100.0, &h, &s);
    let none = build_plan_with_blends(&shapes, &[None], 100.0, &h, &s);
    assert_eq!(plain.plan, none.plan);
}

#[test]
fn 太窄的區域不混色_避免變成條紋() {
    use stitch_gen::design::build_plan_with_blends;
    let shape = ColoredShape {
        polygon: centered_rect(400.0, 20.0), // 2mm 寬
        color: ThreadColor::new(200, 30, 40, "紅"),
    };
    let r = build_plan_with_blends(
        &[shape],
        &[Some([
            ThreadColor::new(200, 30, 40, "紅"),
            ThreadColor::new(40, 80, 180, "藍"),
        ])],
        400.0,
        &hoop("b70-large"),
        &Settings::default(),
    );
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert_eq!(r.plan.colors.len(), 1, "{:?}", r.plan.colors);
}

fn disc(r: f64) -> Polygon {
    Polygon::new(vec![(0..96)
        .map(|i| {
            let a = i as f64 / 96.0 * std::f64::consts::TAU;
            [r * a.cos(), r * a.sin()]
        })
        .collect()])
}

#[test]
fn 圓形用輪廓填充時針點順著邊緣一圈圈往內() {
    use stitch_core::StitchKind;
    let shape = ColoredShape {
        polygon: disc(200.0),
        color: black(),
    };
    let r = build_plan(
        &[shape],
        400.0,
        &hoop("b70-large"),
        &Settings {
            shape_aware: true,
            contour_fill: true,
            ..Settings::default()
        },
    );
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    // 順著邊緣：針點到圓心的距離會一圈圈變小，相鄰兩針的距離變化小
    let pts: Vec<(f64, f64)> = r
        .plan
        .stitches
        .iter()
        .filter(|s| s.kind == StitchKind::Stitch)
        .map(|s| (s.x as f64, s.y as f64))
        .collect();
    let radii: Vec<f64> = pts.iter().map(|p| (p.0 * p.0 + p.1 * p.1).sqrt()).collect();
    // 後半段（輪廓填充）半徑幾乎單調遞減：統計「半徑變小」的比例
    let tail = &radii[radii.len() / 2..];
    let decreasing = tail.windows(2).filter(|w| w[1] <= w[0] + 1.0).count();
    assert!(
        decreasing as f64 / tail.len() as f64 > 0.85,
        "沒有一圈圈往內：{:.2}",
        decreasing as f64 / tail.len() as f64
    );
    validate_plan(&r.plan, &hoop("b70-large")).unwrap();
}

#[test]
fn 關掉輪廓填充時圓形和原本一樣() {
    let shape = || ColoredShape {
        polygon: disc(200.0),
        color: black(),
    };
    let h = hoop("b70-large");
    let a = build_plan(&[shape()], 400.0, &h, &Settings::default());
    let b = build_plan(
        &[shape()],
        400.0,
        &h,
        &Settings {
            contour_fill: false,
            ..Settings::default()
        },
    );
    assert_eq!(a.plan, b.plan);
}

#[test]
fn 細線與輪廓排在填充之後繡_壓在最上面() {
    // 輸入順序故意把細的黑線放在前面：不管面積與輸入順序，線都要等填充繡完才繡
    let black_line = ColoredShape {
        polygon: centered_rect(300.0, 10.0),
        color: ThreadColor::new(0, 0, 0, "黑"),
    };
    let red_fill = ColoredShape {
        polygon: centered_rect(200.0, 200.0),
        color: ThreadColor::new(200, 30, 40, "紅"),
    };
    let r = build_plan(
        &[black_line, red_fill],
        300.0,
        &hoop("b70-large"),
        &Settings {
            shape_aware: true,
            ..Settings::default()
        },
    );
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    let names: Vec<&str> = r.plan.colors.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["紅", "黑"], "線要排在填充之後：{names:?}");
}

#[test]
fn 彎帶開啟順彎填充後針跡不同且仍合法() {
    let n = 48;
    let at = |r: f64, i: usize| {
        let a = (100.0 * i as f64 / n as f64).to_radians();
        [r * a.cos(), r * a.sin()]
    };
    let mut ring: Vec<[f64; 2]> = (0..=n).map(|i| at(400.0, i)).collect();
    ring.extend((0..=n).rev().map(|i| at(300.0, i)));
    let shapes = one(Polygon::new(vec![ring]));
    let run = |flow: bool| {
        build_plan(
            &shapes,
            400.0,
            &hoop("b70-large"),
            &Settings {
                shape_aware: true,
                flow_fill: flow,
                ..Settings::default()
            },
        )
    };
    let (off, on) = (run(false), run(true));
    assert!(on.blocked.is_none(), "{:?}", on.blocked);
    assert_ne!(off.plan.stitches, on.plan.stitches);
    assert!(validate_plan(&on.plan, &hoop("b70-large")).is_ok());
}
