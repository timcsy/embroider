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
