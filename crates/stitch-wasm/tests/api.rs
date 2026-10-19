use stitch_core::hoop::{Hoop, HoopShape};
use stitch_wasm::api;

#[test]
fn 列出內建三個框() {
    let hoops = api::list_builtin_hoops();
    assert_eq!(hoops.len(), 3);
    assert!(hoops.iter().all(|h| h.builtin));
    assert_eq!(hoops[0].id, "b70-large");
}

#[test]
fn 列出已註冊的輸出格式() {
    let formats = api::list_formats();
    let ids: Vec<_> = formats.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(ids, vec!["dst", "exp"]);
    assert_eq!(formats[0].extension, "DST");
    assert!(!formats[0].label.is_empty());
    assert!(!formats[0].description.is_empty());
}

#[test]
fn 驗證框_合法() {
    let h = Hoop {
        id: "mine".into(),
        name: "10×10 方框".into(),
        width: 1000,
        height: 1000,
        shape: HoopShape::Rect,
        margin: 50,
        builtin: false,
    };
    let r = api::validate_hoop(&h);
    assert!(r.ok);
    assert!(r.message.is_none());
}

#[test]
fn 驗證框_不合法時回傳白話訊息() {
    let h = Hoop {
        id: "mine".into(),
        name: "".into(),
        width: 1000,
        height: 1000,
        shape: HoopShape::Rect,
        margin: 50,
        builtin: false,
    };
    let r = api::validate_hoop(&h);
    assert!(!r.ok);
    assert!(!r.message.unwrap().text.is_empty());
}

#[test]
fn 多物件介面可以呼叫且結果合法() {
    use std::path::PathBuf;
    use stitch_compose::{Content, Item, ShapeKind};
    let mut fonts = stitch_text::FontSet::default();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fonts/NotoSansTC-subset.ttf");
    fonts
        .add("sans", "黑體", std::fs::read(path).unwrap())
        .unwrap();
    let hoop = stitch_core::hoop::builtin_hoops().remove(1);
    let req = api::CompositeRequest {
        items: vec![Item {
            id: "a".into(),
            content: Content::Shape {
                shape: ShapeKind::Heart,
                width: 400.0,
                height: 400.0,
                color: stitch_core::ThreadColor::new(200, 30, 30, "正紅"),
                filled: true,
                line_width: 25.0,
            },
            x: 0.0,
            y: 0.0,
            rotation: 0.0,
            scale: 1.0,
        }],
        hoop: hoop.clone(),
        settings: Default::default(),
    };
    let r = api::build_composite_preview(&fonts, &req, &[]);
    assert!(r.preview.blocked.is_none());
    stitch_core::validate_plan(&r.preview.plan, &hoop).unwrap();
}
