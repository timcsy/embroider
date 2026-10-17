use stitch_core::{Point, Stitch, StitchKind, StitchPlan, ThreadColor, UserMessage};

#[test]
fn 針點可以建構並帶有類型() {
    let s = Stitch::stitch(10, -20);
    assert_eq!((s.x, s.y), (10, -20));
    assert_eq!(s.kind, StitchKind::Stitch);
    assert_eq!(Stitch::jump(1, 2).kind, StitchKind::Jump);
    assert_eq!(Stitch::trim(1, 2).kind, StitchKind::Trim);
    assert_eq!(Stitch::color_change(1, 2).kind, StitchKind::ColorChange);
    assert_eq!(Stitch::end(1, 2).kind, StitchKind::End);
}

#[test]
fn 點有座標() {
    let p = Point { x: 3, y: 4 };
    assert_eq!(p, Point::new(3, 4));
}

#[test]
fn 線色帶有中文色名() {
    let c = ThreadColor::new(200, 30, 30, "正紅");
    assert_eq!((c.r, c.g, c.b), (200, 30, 30));
    assert_eq!(c.name, "正紅");
}

#[test]
fn 計畫統計換色次數與針數() {
    let plan = StitchPlan {
        stitches: vec![
            Stitch::stitch(0, 0),
            Stitch::stitch(10, 0),
            Stitch::color_change(10, 0),
            Stitch::stitch(20, 0),
            Stitch::end(20, 0),
        ],
        colors: vec![
            ThreadColor::new(0, 0, 0, "黑"),
            ThreadColor::new(255, 0, 0, "紅"),
        ],
    };
    assert_eq!(plan.color_change_count(), 1);
    assert_eq!(plan.stitch_count(), 3);
}

#[test]
fn 計畫與訊息可以序列化往返() {
    let plan = StitchPlan {
        stitches: vec![Stitch::stitch(1, 2), Stitch::end(1, 2)],
        colors: vec![ThreadColor::new(1, 2, 3, "測試")],
    };
    let json = serde_json::to_string(&plan).unwrap();
    let back: StitchPlan = serde_json::from_str(&json).unwrap();
    assert_eq!(plan, back);

    let m = UserMessage::new("too_big", "這個太大了", Some("請換大一點的框"));
    let json = serde_json::to_string(&m).unwrap();
    let back: UserMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(m, back);
}
