use std::path::PathBuf;
use stitch_geom::{spans_at, Polygon};
use stitch_text::{text_to_polygon, text_to_polygon_styled, Align, FontSet, Layout, TextStyle};

fn fonts() -> FontSet {
    let mut set = FontSet::default();
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fonts/NotoSansTC-subset.ttf");
    set.add("sans", "黑體", std::fs::read(p).unwrap()).unwrap();
    set
}

fn bounds(p: &Polygon) -> ([f64; 2], [f64; 2]) {
    p.bounds().unwrap()
}

/// 某一水平帶（y 範圍）內，字的最左與最右。
fn extent_in_band(p: &Polygon, y0: f64, y1: f64) -> (f64, f64) {
    let mut lo = f64::MAX;
    let mut hi = f64::MIN;
    let mut y = y0;
    while y <= y1 {
        for (a, b) in spans_at(&p.normalized(), y) {
            lo = lo.min(a);
            hi = hi.max(b);
        }
        y += 2.0;
    }
    (lo, hi)
}

fn two_lines(style: TextStyle) -> Polygon {
    // 第一行長、第二行短
    text_to_polygon_styled(
        &fonts(),
        "sans",
        "王小明的家\n王",
        200.0,
        Layout::SingleLine,
        style,
    )
    .unwrap()
    .polygon
}

#[test]
fn 預設樣式與原本的排法完全一樣() {
    let a = text_to_polygon(&fonts(), "sans", "王小明\n的家", 200.0, Layout::SingleLine).unwrap();
    let b = text_to_polygon_styled(
        &fonts(),
        "sans",
        "王小明\n的家",
        200.0,
        Layout::SingleLine,
        TextStyle::default(),
    )
    .unwrap();
    assert_eq!(a.polygon, b.polygon);
}

#[test]
fn 靠左_兩行的左邊緣一樣() {
    let p = two_lines(TextStyle {
        align: Align::Left,
        ..Default::default()
    });
    let (min, max) = bounds(&p);
    let mid = (min[1] + max[1]) / 2.0;
    let first = extent_in_band(&p, min[1], mid - 20.0);
    let second = extent_in_band(&p, mid + 20.0, max[1]);
    assert!((first.0 - second.0).abs() < 25.0, "{first:?} {second:?}");
    assert!(second.1 < first.1 - 200.0);
}

#[test]
fn 靠右_兩行的右邊緣一樣() {
    let p = two_lines(TextStyle {
        align: Align::Right,
        ..Default::default()
    });
    let (min, max) = bounds(&p);
    let mid = (min[1] + max[1]) / 2.0;
    let first = extent_in_band(&p, min[1], mid - 20.0);
    let second = extent_in_band(&p, mid + 20.0, max[1]);
    assert!((first.1 - second.1).abs() < 25.0, "{first:?} {second:?}");
    assert!(second.0 > first.0 + 200.0);
}

#[test]
fn 置中_第二行在第一行的正中間() {
    let p = two_lines(TextStyle::default());
    let (min, max) = bounds(&p);
    let mid = (min[1] + max[1]) / 2.0;
    let first = extent_in_band(&p, min[1], mid - 20.0);
    let second = extent_in_band(&p, mid + 20.0, max[1]);
    let (c1, c2) = ((first.0 + first.1) / 2.0, (second.0 + second.1) / 2.0);
    assert!((c1 - c2).abs() < 40.0, "{c1} {c2}");
}

#[test]
fn 字距加大_整行變寬() {
    let w = |spacing: f64| {
        let t = text_to_polygon_styled(
            &fonts(),
            "sans",
            "王小明的家",
            200.0,
            Layout::SingleLine,
            TextStyle {
                letter_spacing: spacing,
                ..Default::default()
            },
        )
        .unwrap();
        let (min, max) = bounds(&t.polygon);
        max[0] - min[0]
    };
    let (a, b) = (w(0.0), w(0.5));
    // 五個字，四個間隔，每個多 0.5 個字高 = 100
    assert!((b - a - 400.0).abs() < 30.0, "{a} → {b}");
    // 字距收緊會變窄，但不會把字疊成一團
    assert!(w(-0.1) < a);
}

#[test]
fn 行距加大_整段變高() {
    let h = |lh: f64| {
        let t = text_to_polygon_styled(
            &fonts(),
            "sans",
            "王\n小\n明",
            200.0,
            Layout::SingleLine,
            TextStyle {
                line_height: lh,
                ..Default::default()
            },
        )
        .unwrap();
        let (min, max) = bounds(&t.polygon);
        max[1] - min[1]
    };
    assert!(h(2.0) > h(1.3) + 250.0, "{} {}", h(1.3), h(2.0));
    assert!(h(1.0) < h(1.3));
}

#[test]
fn 自動換行也算字距() {
    let lines = |spacing: f64| {
        let t = text_to_polygon_styled(
            &fonts(),
            "sans",
            "王小明的家人朋友",
            200.0,
            Layout::Wrap { max_width: 1000.0 },
            TextStyle {
                letter_spacing: spacing,
                ..Default::default()
            },
        )
        .unwrap();
        let (min, max) = bounds(&t.polygon);
        max[1] - min[1]
    };
    // 字距加大之後同樣的寬度放不下，會多換一行
    assert!(lines(0.6) > lines(0.0));
}

#[test]
fn 不合理的字距行距被限制在安全範圍() {
    let t = text_to_polygon_styled(
        &fonts(),
        "sans",
        "王小明",
        200.0,
        Layout::SingleLine,
        TextStyle {
            letter_spacing: -5.0,
            line_height: 0.0,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(bounds(&t.polygon).1[0] > bounds(&t.polygon).0[0]);
}
