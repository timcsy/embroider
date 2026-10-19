use std::path::PathBuf;
use stitch_compose::{build_composite, CompositeRequest, Content, Item, ShapeKind};
use stitch_core::hoop::{builtin_hoops, Hoop};
use stitch_core::{validate_plan, StitchKind, ThreadColor};
use stitch_text::FontSet;

fn fonts() -> FontSet {
    let mut set = FontSet::default();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fonts/NotoSansTC-subset.ttf");
    set.add("sans", "黑體", std::fs::read(path).unwrap())
        .unwrap();
    set
}

fn hoop(id: &str) -> Hoop {
    builtin_hoops().into_iter().find(|h| h.id == id).unwrap()
}

fn red() -> ThreadColor {
    ThreadColor::new(200, 30, 30, "正紅")
}

fn text(id: &str, s: &str, x: f64, y: f64) -> Item {
    Item {
        id: id.into(),
        content: Content::Text {
            text: s.into(),
            font_id: "sans".into(),
            height: 150.0,
            color: red(),
            max_width: None,
        },
        x,
        y,
        rotation: 0.0,
        scale: 1.0,
    }
}

fn circle(id: &str, d: f64, x: f64, y: f64) -> Item {
    Item {
        id: id.into(),
        content: Content::Shape {
            shape: ShapeKind::Circle,
            width: d,
            height: d,
            color: ThreadColor::new(30, 60, 200, "寶藍"),
            filled: true,
            line_width: 20.0,
        },
        x,
        y,
        rotation: 0.0,
        scale: 1.0,
    }
}

fn req(items: Vec<Item>, h: &str) -> CompositeRequest {
    CompositeRequest {
        items,
        hoop: hoop(h),
        settings: Default::default(),
    }
}

/// 針點（只算下針）的外框
fn bounds(r: &stitch_compose::CompositeResult) -> (f64, f64, f64, f64) {
    let pts: Vec<_> = r
        .preview
        .plan
        .stitches
        .iter()
        .filter(|s| s.kind == StitchKind::Stitch)
        .collect();
    let x0 = pts.iter().map(|s| s.x).min().unwrap() as f64;
    let x1 = pts.iter().map(|s| s.x).max().unwrap() as f64;
    let y0 = pts.iter().map(|s| s.y).min().unwrap() as f64;
    let y1 = pts.iter().map(|s| s.y).max().unwrap() as f64;
    (x0, y0, x1, y1)
}

#[test]
fn 沒有物件就說還沒有內容() {
    let r = build_composite(&fonts(), &req(vec![], "b70-medium"), &[]);
    assert_eq!(r.preview.blocked.unwrap().code, "empty_design");
}

#[test]
fn 兩個物件各在自己的位置() {
    let items = vec![text("a", "上", 0.0, -300.0), text("b", "下", 0.0, 300.0)];
    let r = build_composite(&fonts(), &req(items, "b70-large"), &[]);
    assert!(r.preview.blocked.is_none(), "{:?}", r.preview.blocked);
    let (_, y0, _, y1) = bounds(&r);
    assert!(y0 < -200.0 && y1 > 200.0, "兩個字應分在上下：{y0} {y1}");
    // 兩個物件同色，只有一個色塊
    assert_eq!(r.preview.plan.colors.len(), 1);
}

#[test]
fn 不同顏色的物件各有各的線色() {
    let items = vec![text("a", "字", -200.0, 0.0), circle("c", 300.0, 200.0, 0.0)];
    let r = build_composite(&fonts(), &req(items, "b70-large"), &[]);
    assert_eq!(r.preview.plan.colors.len(), 2);
}

#[test]
fn 移動物件針跡跟著移動() {
    let at = |x: f64| {
        build_composite(
            &fonts(),
            &req(vec![circle("c", 300.0, x, 0.0)], "b70-large"),
            &[],
        )
    };
    let (a, b) = (bounds(&at(0.0)), bounds(&at(400.0)));
    assert!((b.0 - a.0 - 400.0).abs() < 15.0, "{a:?} {b:?}");
    assert!((b.2 - a.2 - 400.0).abs() < 15.0);
}

#[test]
fn 旋轉九十度寬高對調() {
    let wide = |rot: f64| {
        let mut it = Item {
            id: "r".into(),
            content: Content::Shape {
                shape: ShapeKind::Rect,
                width: 500.0,
                height: 100.0,
                color: red(),
                filled: true,
                line_width: 20.0,
            },
            x: 0.0,
            y: 0.0,
            rotation: rot,
            scale: 1.0,
        };
        it.rotation = rot;
        build_composite(&fonts(), &req(vec![it], "b70-large"), &[])
    };
    let (a, b) = (bounds(&wide(0.0)), bounds(&wide(90.0)));
    assert!((a.2 - a.0) > 450.0 && (a.3 - a.1) < 150.0, "{a:?}");
    assert!((b.3 - b.1) > 450.0 && (b.2 - b.0) < 150.0, "{b:?}");
}

#[test]
fn 縮放比例改變物件大小() {
    let mut c = circle("c", 300.0, 0.0, 0.0);
    c.scale = 2.0;
    let r = build_composite(&fonts(), &req(vec![c], "b70-large"), &[]);
    let (x0, _, x1, _) = bounds(&r);
    assert!((x1 - x0 - 600.0).abs() < 40.0, "{}", x1 - x0);
    assert!((r.items[0].width - 600.0).abs() < 1.0);
}

#[test]
fn 太小的字會放大並回報實際大小() {
    let mut t = text("a", "小", 0.0, 0.0);
    t.scale = 0.1;
    let r = build_composite(&fonts(), &req(vec![t], "b70-large"), &[]);
    assert!(r.preview.blocked.is_none());
    assert!(r.items[0].scale > 0.1 + 1e-6, "{}", r.items[0].scale);
    assert!(r.preview.adjustments.iter().any(|m| m.code == "enlarged"));
}

#[test]
fn 超出框會縮小整個設計_相對位置不變() {
    // 兩個字放在框的兩端之外
    let items = vec![text("a", "左", -1500.0, 0.0), text("b", "右", 1500.0, 0.0)];
    let r = build_composite(&fonts(), &req(items, "b70-medium"), &[]);
    assert!(r.preview.blocked.is_none(), "{:?}", r.preview.blocked);
    assert!(r.preview.fitted_scale < 1.0);
    assert!(validate_plan(&r.preview.plan, &hoop("b70-medium")).is_ok());
}

#[test]
fn 縮到字太小就擋下() {
    // 很多字、位置極遠，縮到最小字高以下
    let items = vec![
        text("a", "左", -20000.0, 0.0),
        text("b", "右", 20000.0, 0.0),
    ];
    let r = build_composite(&fonts(), &req(items, "b70-small"), &[]);
    assert_eq!(r.preview.blocked.unwrap().code, "too_big_for_hoop");
}

#[test]
fn 空白文字的物件被略過_其他物件照常() {
    let items = vec![text("a", "   ", 0.0, 0.0), circle("c", 300.0, 0.0, 0.0)];
    let r = build_composite(&fonts(), &req(items, "b70-large"), &[]);
    assert!(r.preview.blocked.is_none());
    assert!(r.items[0].problem.is_some());
    assert!(r.items[1].problem.is_none());
}

#[test]
fn 空心形狀中間是空的() {
    let ring = Item {
        id: "o".into(),
        content: Content::Shape {
            shape: ShapeKind::Circle,
            width: 600.0,
            height: 600.0,
            color: red(),
            filled: false,
            line_width: 30.0,
        },
        x: 0.0,
        y: 0.0,
        rotation: 0.0,
        scale: 1.0,
    };
    let r = build_composite(&fonts(), &req(vec![ring], "b70-large"), &[]);
    assert!(r.preview.blocked.is_none(), "{:?}", r.preview.blocked);
    let inside = r
        .preview
        .plan
        .stitches
        .iter()
        .filter(|s| s.kind == StitchKind::Stitch && ((s.x * s.x + s.y * s.y) as f64).sqrt() < 200.0)
        .count();
    assert_eq!(inside, 0, "中間不該有針");
}

#[test]
fn 每種形狀都能繡() {
    for shape in [
        ShapeKind::Circle,
        ShapeKind::Rect,
        ShapeKind::Heart,
        ShapeKind::Star,
        ShapeKind::Line,
    ] {
        let it = Item {
            id: "s".into(),
            content: Content::Shape {
                shape,
                width: 400.0,
                height: 400.0,
                color: red(),
                filled: true,
                line_width: 25.0,
            },
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale: 1.0,
        };
        let r = build_composite(&fonts(), &req(vec![it], "b70-large"), &[]);
        assert!(
            r.preview.blocked.is_none(),
            "{shape:?} {:?}",
            r.preview.blocked
        );
        assert!(!r.preview.plan.stitches.is_empty());
    }
}

#[test]
fn 相同輸入得到相同輸出() {
    let items = vec![text("a", "字", 0.0, -200.0), circle("c", 300.0, 0.0, 200.0)];
    let a = build_composite(&fonts(), &req(items.clone(), "b70-large"), &[]);
    let b = build_composite(&fonts(), &req(items, "b70-large"), &[]);
    assert_eq!(a.preview.plan, b.preview.plan);
}

#[test]
fn 圖片與文字放在同一個設計() {
    let rgba = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../stitch-image/tests/fixtures/thanks-256.rgba"),
    )
    .unwrap();
    let image = Item {
        id: "img".into(),
        content: Content::Image {
            image: 0,
            width: 256,
            height: 171,
            color_count: 6,
            simplify: 50,
            size: 1000.0,
        },
        x: 0.0,
        y: -200.0,
        rotation: 0.0,
        scale: 1.0,
    };
    let items = vec![image, text("t", "感恩", 0.0, 650.0)];
    let r = build_composite(&fonts(), &req(items, "b70-large"), &[rgba]);
    assert!(r.preview.blocked.is_none(), "{:?}", r.preview.blocked);
    assert!(r.items.iter().all(|i| i.problem.is_none()), "{:?}", r.items);
    assert!(validate_plan(&r.preview.plan, &hoop("b70-large")).is_ok());
    let (_, y0, _, y1) = bounds(&r);
    assert!(
        y1 - y0 > 1000.0,
        "圖在上、字在下，總高應該很大：{}",
        y1 - y0
    );
}

#[test]
fn 圖片壞掉只影響那個物件() {
    let image = Item {
        id: "img".into(),
        content: Content::Image {
            image: 5,
            width: 4,
            height: 4,
            color_count: 4,
            simplify: 30,
            size: 800.0,
        },
        x: 0.0,
        y: 0.0,
        rotation: 0.0,
        scale: 1.0,
    };
    let r = build_composite(
        &fonts(),
        &req(vec![image, circle("c", 300.0, 0.0, 0.0)], "b70-large"),
        &[],
    );
    assert!(r.preview.blocked.is_none());
    assert!(r.items[0].problem.is_some());
}
