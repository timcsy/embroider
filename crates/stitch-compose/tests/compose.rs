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
            curve: 0.0,
            align: stitch_compose::TextAlign::Center,
            letter_spacing: 0.0,
            line_height: 1.3,
        },
        x,
        y,
        rotation: 0.0,
        scale: 1.0,
        flip_x: false,
        flip_y: false,
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
        flip_x: false,
        flip_y: false,
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
            flip_x: false,
            flip_y: false,
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
    let items = vec![text("a", "左", -540.0, 0.0), text("b", "右", 540.0, 0.0)];
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
        flip_x: false,
        flip_y: false,
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
            flip_x: false,
            flip_y: false,
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
        flip_x: false,
        flip_y: false,
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
        flip_x: false,
        flip_y: false,
    };
    let r = build_composite(
        &fonts(),
        &req(vec![image, circle("c", 300.0, 0.0, 0.0)], "b70-large"),
        &[],
    );
    assert!(r.preview.blocked.is_none());
    assert!(r.items[0].problem.is_some());
}

#[test]
fn 同一張圖移動後不必重算_第二次明顯更快() {
    let rgba = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../stitch-image/tests/fixtures/thanks-256.rgba"),
    )
    .unwrap();
    let make = |x: f64| Item {
        id: "img".into(),
        content: Content::Image {
            image: 0,
            width: 256,
            height: 171,
            color_count: 5,
            simplify: 40,
            size: 800.0,
        },
        x,
        y: 0.0,
        rotation: 0.0,
        scale: 1.0,
        flip_x: false,
        flip_y: false,
    };
    let run = |x: f64| {
        let t = std::time::Instant::now();
        let r = build_composite(
            &fonts(),
            &req(vec![make(x)], "b70-large"),
            std::slice::from_ref(&rgba),
        );
        assert!(r.preview.blocked.is_none());
        t.elapsed()
    };
    let first = run(0.0);
    let second = run(100.0);
    // 第二次只需要鋪針，不必再轉圖
    assert!(second < first, "{first:?} → {second:?}");
}

#[test]
fn 七個字放不進小框會被擋下並說明() {
    let mut t = text("a", "王小明的刺繡作", 0.0, 0.0);
    if let Content::Text { height, .. } = &mut t.content {
        *height = 250.0;
    }
    let r = build_composite(&fonts(), &req(vec![t], "b70-small"), &[]);
    assert_eq!(
        r.preview.blocked.map(|m| m.code),
        Some("too_big_for_hoop".into()),
        "{:?}",
        r.preview.adjustments
    );
}

#[test]
fn 一行放不下的長句自動分行並說明() {
    let long = "王小明的刺繡作品展覽很長的一段文字";
    let r = build_composite(
        &fonts(),
        &req(vec![text("a", long, 0.0, 0.0)], "b70-large"),
        &[],
    );
    assert!(r.preview.blocked.is_none(), "{:?}", r.preview.blocked);
    assert!(
        r.preview.adjustments.iter().any(|m| m.code == "wrapped"),
        "{:?}",
        r.preview.adjustments
    );
    // 分行後比單行高
    let (_, y0, _, y1) = bounds(&r);
    assert!(y1 - y0 > 300.0, "高度 {}", y1 - y0);
}

#[test]
fn 短的名字不會被分行() {
    let r = build_composite(
        &fonts(),
        &req(vec![text("a", "王小明", 0.0, 0.0)], "b70-medium"),
        &[],
    );
    assert!(!r.preview.adjustments.iter().any(|m| m.code == "wrapped"));
}

fn draw(strokes: Vec<Vec<[f64; 2]>>, width: f64, filled: bool) -> Item {
    Item {
        id: "d".into(),
        content: Content::Draw {
            strokes,
            width,
            color: red(),
            filled,
        },
        x: 0.0,
        y: 0.0,
        rotation: 0.0,
        scale: 1.0,
        flip_x: false,
        flip_y: false,
    }
}

#[test]
fn 手繪直線_粗細與長度相符() {
    let it = draw(vec![vec![[-300.0, 0.0], [300.0, 0.0]]], 30.0, false);
    let r = build_composite(&fonts(), &req(vec![it], "b70-large"), &[]);
    assert!(r.preview.blocked.is_none(), "{:?}", r.preview.blocked);
    let (x0, y0, x1, y1) = bounds(&r);
    assert!((x1 - x0 - 600.0).abs() < 60.0, "長 {}", x1 - x0);
    assert!(y1 - y0 < 60.0, "粗 {}", y1 - y0);
    assert!(
        (r.items[0].width - 630.0).abs() < 40.0,
        "{}",
        r.items[0].width
    );
}

#[test]
fn 手繪很細的線走單線平針_針數很少() {
    let thin = draw(vec![vec![[-300.0, 0.0], [300.0, 0.0]]], 6.0, false);
    let thick = draw(vec![vec![[-300.0, 0.0], [300.0, 0.0]]], 30.0, false);
    let n = |it: &Item| {
        build_composite(&fonts(), &req(vec![it.clone()], "b70-large"), &[])
            .preview
            .plan
            .stitches
            .len()
    };
    assert!(n(&thin) < n(&thick), "細線 {} 粗線 {}", n(&thin), n(&thick));
}

#[test]
fn 手繪抖動的線會被抹順_仍然合法() {
    let wobbly: Vec<[f64; 2]> = (0..80)
        .map(|i| {
            let x = i as f64 * 8.0 - 320.0;
            [x, (i as f64 * 1.7).sin() * 4.0 + (i % 2) as f64 * 3.0]
        })
        .collect();
    let r = build_composite(
        &fonts(),
        &req(vec![draw(vec![wobbly], 25.0, false)], "b70-large"),
        &[],
    );
    assert!(r.preview.blocked.is_none(), "{:?}", r.preview.blocked);
    assert!(validate_plan(&r.preview.plan, &hoop("b70-large")).is_ok());
}

fn circle_stroke(r: f64) -> Vec<[f64; 2]> {
    (0..=48)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / 48.0;
            [r * a.cos(), r * a.sin()]
        })
        .collect()
}

fn stitches_inside(r: &stitch_compose::CompositeResult, radius: f64) -> usize {
    r.preview
        .plan
        .stitches
        .iter()
        .filter(|s| {
            s.kind == StitchKind::Stitch && ((s.x * s.x + s.y * s.y) as f64).sqrt() < radius
        })
        .count()
}

#[test]
fn 手繪封閉圖形選填滿才會填中間() {
    let hollow = build_composite(
        &fonts(),
        &req(
            vec![draw(vec![circle_stroke(250.0)], 25.0, false)],
            "b70-large",
        ),
        &[],
    );
    let filled = build_composite(
        &fonts(),
        &req(
            vec![draw(vec![circle_stroke(250.0)], 25.0, true)],
            "b70-large",
        ),
        &[],
    );
    assert_eq!(stitches_inside(&hollow, 150.0), 0, "空心中間不該有針");
    assert!(stitches_inside(&filled, 150.0) > 50, "填滿中間要有針");
}

#[test]
fn 手繪沒封起來的線就算選填滿也不填() {
    let arc: Vec<[f64; 2]> = circle_stroke(250.0).into_iter().take(30).collect();
    let r = build_composite(
        &fonts(),
        &req(vec![draw(vec![arc], 25.0, true)], "b70-large"),
        &[],
    );
    assert_eq!(stitches_inside(&r, 150.0), 0);
}

#[test]
fn 手繪只點一下變成一個圓點() {
    let r = build_composite(
        &fonts(),
        &req(vec![draw(vec![vec![[0.0, 0.0]]], 40.0, false)], "b70-large"),
        &[],
    );
    assert!(r.preview.blocked.is_none(), "{:?}", r.preview.blocked);
    assert!(!r.preview.plan.stitches.is_empty());
}

#[test]
fn 手繪多筆都繡出來() {
    let strokes = vec![
        vec![[-300.0, -200.0], [300.0, -200.0]],
        vec![[-300.0, 200.0], [300.0, 200.0]],
    ];
    let r = build_composite(
        &fonts(),
        &req(vec![draw(strokes, 30.0, false)], "b70-large"),
        &[],
    );
    let (_, y0, _, y1) = bounds(&r);
    assert!(y1 - y0 > 350.0, "兩條線要分開：{}", y1 - y0);
}

#[test]
fn 手繪沒有筆畫時回報這個物件的問題() {
    let r = build_composite(
        &fonts(),
        &req(
            vec![draw(vec![], 30.0, false), circle("c", 300.0, 0.0, 0.0)],
            "b70-large",
        ),
        &[],
    );
    assert!(r.items[0].problem.is_some());
    assert!(r.preview.blocked.is_none());
}

#[test]
fn 手繪可以旋轉與縮放() {
    let mut it = draw(vec![vec![[-300.0, 0.0], [300.0, 0.0]]], 30.0, false);
    it.rotation = 90.0;
    it.scale = 0.5;
    let r = build_composite(&fonts(), &req(vec![it], "b70-large"), &[]);
    let (x0, y0, x1, y1) = bounds(&r);
    assert!((y1 - y0) > 250.0 && (x1 - x0) < 60.0, "{x0} {y0} {x1} {y1}");
}

fn flipped(mut it: Item, fx: bool, fy: bool) -> Item {
    it.flip_x = fx;
    it.flip_y = fy;
    it
}

fn left_heavy() -> Item {
    // 左邊粗、右邊細的長條：用外框的不對稱來看有沒有翻過去
    Item {
        id: "lh".into(),
        content: Content::Draw {
            strokes: vec![
                vec![[-300.0, 0.0], [300.0, 0.0]],
                vec![[-300.0, -150.0], [-300.0, 150.0]],
            ],
            width: 30.0,
            color: red(),
            filled: false,
        },
        x: 0.0,
        y: 0.0,
        rotation: 0.0,
        scale: 1.0,
        flip_x: false,
        flip_y: false,
    }
}

fn mean_x(r: &stitch_compose::CompositeResult) -> f64 {
    let pts: Vec<f64> = r
        .preview
        .plan
        .stitches
        .iter()
        .filter(|s| s.kind == StitchKind::Stitch)
        .map(|s| s.x as f64)
        .collect();
    pts.iter().sum::<f64>() / pts.len() as f64
}

#[test]
fn 左右翻轉後重心換到另一邊() {
    let a = build_composite(&fonts(), &req(vec![left_heavy()], "b70-large"), &[]);
    let b = build_composite(
        &fonts(),
        &req(vec![flipped(left_heavy(), true, false)], "b70-large"),
        &[],
    );
    assert!(mean_x(&a) < -10.0, "{}", mean_x(&a));
    assert!(mean_x(&b) > 10.0, "{}", mean_x(&b));
}

#[test]
fn 上下翻轉後重心換到另一邊() {
    let mean_y = |r: &stitch_compose::CompositeResult| {
        let pts: Vec<f64> = r
            .preview
            .plan
            .stitches
            .iter()
            .filter(|s| s.kind == StitchKind::Stitch)
            .map(|s| s.y as f64)
            .collect();
        pts.iter().sum::<f64>() / pts.len() as f64
    };
    let mut it = left_heavy();
    if let Content::Draw { strokes, .. } = &mut it.content {
        strokes.push(vec![[250.0, 0.0], [250.0, 200.0]]); // 右邊再往下加一條，上下不對稱
    }
    let a = build_composite(&fonts(), &req(vec![it.clone()], "b70-large"), &[]);
    let b = build_composite(
        &fonts(),
        &req(vec![flipped(it, false, true)], "b70-large"),
        &[],
    );
    assert!(
        mean_y(&a) * mean_y(&b) < 0.0 || (mean_y(&a) - mean_y(&b)).abs() > 20.0,
        "{} {}",
        mean_y(&a),
        mean_y(&b)
    );
}

fn curved(text_: &str, curve: f64) -> Item {
    let mut t = text("c", text_, 0.0, 0.0);
    if let Content::Text { curve: c, .. } = &mut t.content {
        *c = curve;
    }
    t
}

#[test]
fn 字繞成拱形_兩端比中間低() {
    let flat = build_composite(
        &fonts(),
        &req(vec![curved("王小明王小明", 0.0)], "b70-large"),
        &[],
    );
    let arch = build_composite(
        &fonts(),
        &req(vec![curved("王小明王小明", 120.0)], "b70-large"),
        &[],
    );
    assert!(arch.preview.blocked.is_none(), "{:?}", arch.preview.blocked);
    let h = |r: &stitch_compose::CompositeResult| {
        let (_, y0, _, y1) = bounds(r);
        y1 - y0
    };
    assert!(
        h(&arch) > h(&flat) * 2.0,
        "拱形比較高 {} vs {}",
        h(&arch),
        h(&flat)
    );
    // 左半邊最下面的針，比正中間最下面的針更低
    let ys: Vec<(i32, i32)> = arch
        .preview
        .plan
        .stitches
        .iter()
        .filter(|s| s.kind == StitchKind::Stitch)
        .map(|s| (s.x, s.y))
        .collect();
    let low = |lo: i32, hi: i32| {
        ys.iter()
            .filter(|p| p.0 >= lo && p.0 < hi)
            .map(|p| p.1)
            .max()
            .unwrap()
    };
    let (x0, _, x1, _) = bounds(&arch);
    let (x0, x1) = (x0 as i32, x1 as i32);
    let w = x1 - x0;
    assert!(low(x0, x0 + w / 6) > low(x0 + w * 2 / 5, x0 + w * 3 / 5) + 100);
}

#[test]
fn 字繞成笑臉形_兩端比中間高() {
    let smile = build_composite(
        &fonts(),
        &req(vec![curved("王小明王小明", -120.0)], "b70-large"),
        &[],
    );
    let ys: Vec<(i32, i32)> = smile
        .preview
        .plan
        .stitches
        .iter()
        .filter(|s| s.kind == StitchKind::Stitch)
        .map(|s| (s.x, s.y))
        .collect();
    let (x0, _, x1, _) = bounds(&smile);
    let (x0, x1) = (x0 as i32, x1 as i32);
    let w = x1 - x0;
    let high = |lo: i32, hi: i32| {
        ys.iter()
            .filter(|p| p.0 >= lo && p.0 < hi)
            .map(|p| p.1)
            .min()
            .unwrap()
    };
    assert!(high(x0, x0 + w / 6) < high(x0 + w * 2 / 5, x0 + w * 3 / 5) - 100);
}

#[test]
fn 弧形文字也能旋轉與放進框() {
    let mut it = curved("王小明王小明", 90.0);
    it.rotation = 30.0;
    let r = build_composite(&fonts(), &req(vec![it], "b70-medium"), &[]);
    assert!(r.preview.blocked.is_none(), "{:?}", r.preview.blocked);
    assert!(validate_plan(&r.preview.plan, &hoop("b70-medium")).is_ok());
}

#[test]
fn 文字壓在圖案上面會提醒_但不擋() {
    let items = vec![circle("c", 600.0, 0.0, 0.0), text("t", "王小明", 0.0, 0.0)];
    let r = build_composite(&fonts(), &req(items, "b70-large"), &[]);
    assert!(r.preview.blocked.is_none(), "{:?}", r.preview.blocked);
    let m = r
        .preview
        .adjustments
        .iter()
        .find(|m| m.code == "text_overlap")
        .expect("應該有提醒");
    assert!(m.text.contains("文字"), "{}", m.text);
}

#[test]
fn 文字和圖案分開放就不提醒() {
    let items = vec![
        circle("c", 300.0, 0.0, -500.0),
        text("t", "王小明", 0.0, 400.0),
    ];
    let r = build_composite(&fonts(), &req(items, "b70-large"), &[]);
    assert!(!r
        .preview
        .adjustments
        .iter()
        .any(|m| m.code == "text_overlap"));
}

#[test]
fn 文字壓在文字上也提醒一次就好() {
    let items = vec![
        text("a", "王小明", 0.0, 0.0),
        text("b", "王小明", 10.0, 10.0),
    ];
    let r = build_composite(&fonts(), &req(items, "b70-large"), &[]);
    assert_eq!(
        r.preview
            .adjustments
            .iter()
            .filter(|m| m.code == "text_overlap")
            .count(),
        1
    );
}

fn elapsed_ms(items: Vec<Item>, images: &[Vec<u8>]) -> u128 {
    let t = std::time::Instant::now();
    let r = build_composite(&fonts(), &req(items, "b70-large"), images);
    eprintln!(
        "耗時 {:?} blocked={:?}",
        t.elapsed(),
        r.preview.blocked.as_ref().map(|m| &m.code)
    );
    t.elapsed().as_millis()
}

#[test]
#[ignore = "量測用：cargo test --release -- --ignored --nocapture"]
fn 量測_二十四個愛心() {
    let items: Vec<Item> = (0..24)
        .map(|i| {
            let mut c = circle(
                &format!("h{i}"),
                150.0,
                (i % 6) as f64 * 200.0 - 500.0,
                (i / 6) as f64 * 200.0 - 300.0,
            );
            if let Content::Shape { shape, .. } = &mut c.content {
                *shape = ShapeKind::Heart;
            }
            c
        })
        .collect();
    elapsed_ms(items, &[]);
}

#[test]
#[ignore = "量測用"]
fn 量測_十二個同一張圖() {
    let rgba = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../stitch-image/tests/fixtures/thanks-256.rgba"),
    )
    .unwrap();
    let items: Vec<Item> = (0..12)
        .map(|i| Item {
            id: format!("i{i}"),
            content: Content::Image {
                image: 0,
                width: 256,
                height: 171,
                color_count: 5,
                simplify: 40,
                size: 400.0,
            },
            x: (i % 4) as f64 * 350.0 - 500.0,
            y: (i / 4) as f64 * 250.0 - 300.0,
            rotation: 0.0,
            scale: 1.0,
            flip_x: false,
            flip_y: false,
        })
        .collect();
    elapsed_ms(items, std::slice::from_ref(&rgba));
}

#[test]
#[ignore = "量測用"]
fn 量測_五千點的長手繪() {
    let pts: Vec<[f64; 2]> = (0..5000)
        .map(|i| {
            let a = i as f64 * 0.01;
            [a.cos() * (100.0 + a * 4.0), a.sin() * (100.0 + a * 4.0)]
        })
        .collect();
    elapsed_ms(vec![draw(vec![pts], 25.0, false)], &[]);
}

fn styled(text_: &str, align: stitch_compose::TextAlign, spacing: f64, lh: f64) -> Item {
    let mut t = text("s", text_, 0.0, 0.0);
    if let Content::Text {
        align: a,
        letter_spacing: ls,
        line_height: l,
        ..
    } = &mut t.content
    {
        *a = align;
        *ls = spacing;
        *l = lh;
    }
    t
}

#[test]
fn 字距加大整行變寬() {
    let w = |sp: f64| {
        let r = build_composite(
            &fonts(),
            &req(
                vec![styled("王小明", stitch_compose::TextAlign::Center, sp, 1.3)],
                "b70-large",
            ),
            &[],
        );
        r.items[0].width
    };
    assert!(w(0.4) > w(0.0) + 100.0, "{} {}", w(0.0), w(0.4));
}

#[test]
fn 靠左與置中的多行文字外框一樣但針跡位置不同() {
    let mean = |a: stitch_compose::TextAlign| {
        let r = build_composite(
            &fonts(),
            &req(vec![styled("王小明的家\n王", a, 0.0, 1.3)], "b70-large"),
            &[],
        );
        assert!(r.preview.blocked.is_none(), "{:?}", r.preview.blocked);
        // 第二行（較短）在下半部：看下半部針的平均 x
        let ys: Vec<i32> = r
            .preview
            .plan
            .stitches
            .iter()
            .filter(|s| s.kind == StitchKind::Stitch)
            .map(|s| s.y)
            .collect();
        let mid = (ys.iter().min().unwrap() + ys.iter().max().unwrap()) / 2;
        let xs: Vec<f64> = r
            .preview
            .plan
            .stitches
            .iter()
            .filter(|s| s.kind == StitchKind::Stitch && s.y > mid + 50)
            .map(|s| s.x as f64)
            .collect();
        xs.iter().sum::<f64>() / xs.len() as f64
    };
    let (left, center, right) = (
        mean(stitch_compose::TextAlign::Left),
        mean(stitch_compose::TextAlign::Center),
        mean(stitch_compose::TextAlign::Right),
    );
    assert!(left < center - 50.0, "{left} {center}");
    assert!(right > center + 50.0, "{right} {center}");
}

#[test]
fn 行距加大整段變高() {
    let h = |lh: f64| {
        build_composite(
            &fonts(),
            &req(
                vec![styled(
                    "王\n小\n明",
                    stitch_compose::TextAlign::Center,
                    0.0,
                    lh,
                )],
                "b70-large",
            ),
            &[],
        )
        .items[0]
            .height
    };
    assert!(h(2.0) > h(1.3) + 200.0);
}

#[test]
fn 每個物件都有自己的針跡層_順序照上下層() {
    let items = vec![text("t", "王", -200.0, 0.0), circle("c", 300.0, 200.0, 0.0)];
    let r = build_composite(&fonts(), &req(items, "b70-large"), &[]);
    assert_eq!(
        r.layers.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(),
        ["t", "c"]
    );
    for l in &r.layers {
        assert!(l.parts.iter().all(|p| !p.runs.is_empty()));
    }
    // 層裡的針點總數等於預覽計畫裡的下針數（只是還沒接上跳針與固定針）
    let points: usize = r
        .layers
        .iter()
        .flat_map(|l| &l.parts)
        .flat_map(|p| &p.runs)
        .map(Vec::len)
        .sum();
    let stitches = r
        .preview
        .plan
        .stitches
        .iter()
        .filter(|s| s.kind == StitchKind::Stitch)
        .count();
    assert!(
        points > 0 && points <= stitches + 10 && stitches <= points + 40 * r.layers.len() * 8,
        "{points} {stitches}"
    );
    // 預覽結果本身不再帶重複的層
    assert!(r.preview.layers.is_empty());
}

#[test]
fn 移動物件_它的針跡層跟著平移同樣的距離() {
    let first = |x: f64| {
        let r = build_composite(
            &fonts(),
            &req(vec![circle("c", 300.0, x, 0.0)], "b70-large"),
            &[],
        );
        r.layers[0].parts[0].runs[0][0]
    };
    let (a, b) = (first(0.0), first(300.0));
    assert!((b[0] - a[0] - 300.0).abs() < 20.0, "{a:?} {b:?}");
    assert!((b[1] - a[1]).abs() < 20.0);
}

#[test]
fn 有問題的物件沒有針跡層() {
    let items = vec![text("blank", "   ", 0.0, 0.0), circle("c", 300.0, 0.0, 0.0)];
    let r = build_composite(&fonts(), &req(items, "b70-large"), &[]);
    assert_eq!(
        r.layers.iter().map(|l| l.id.as_str()).collect::<Vec<_>>(),
        ["c"]
    );
}

#[test]
fn 混色圖片的一個物件有兩種線色的層() {
    // 一般形狀：只有一個顏色的層
    let r = build_composite(
        &fonts(),
        &req(vec![circle("c", 400.0, 0.0, 0.0)], "b70-large"),
        &[],
    );
    assert_eq!(
        r.layers[0]
            .parts
            .iter()
            .map(|p| p.color.name.clone())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        1
    );
}
