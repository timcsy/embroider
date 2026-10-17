use proptest::prelude::*;
use stitch_core::hoop::{builtin_hoops, validate_hoop, Hoop, HoopShape};
use stitch_core::Point;

fn rect(w: i32, h: i32) -> Hoop {
    Hoop {
        id: "t".into(),
        name: "測試框".into(),
        width: w,
        height: h,
        shape: HoopShape::Rect,
        margin: 50,
        builtin: false,
    }
}

#[test]
fn 內建三個框尺寸正確且不可刪除() {
    let hoops = builtin_hoops();
    let find = |id: &str| hoops.iter().find(|h| h.id == id).unwrap().clone();
    let l = find("b70-large");
    assert_eq!((l.width, l.height), (1600, 2600));
    let m = find("b70-medium");
    assert_eq!((m.width, m.height), (1200, 1800));
    let s = find("b70-small");
    assert_eq!((s.width, s.height), (500, 750));
    for h in &hoops {
        assert!(h.builtin);
        assert!(!h.can_delete());
        assert_eq!(h.margin, 50);
        assert!(validate_hoop(h).is_ok());
    }
}

#[test]
fn 名稱必須非空且不超過20字() {
    let mut h = rect(1000, 1000);
    h.name = "   ".into();
    assert!(validate_hoop(&h).is_err());
    h.name = "一".repeat(20);
    assert!(validate_hoop(&h).is_ok());
    h.name = "一".repeat(21);
    assert!(validate_hoop(&h).is_err());
}

#[test]
fn 寬高必須大於0且不超過3000() {
    assert!(validate_hoop(&rect(0, 1000)).is_err());
    assert!(validate_hoop(&rect(1000, 0)).is_err());
    assert!(validate_hoop(&rect(-5, 1000)).is_err());
    assert!(validate_hoop(&rect(3000, 3000)).is_ok());
    assert!(validate_hoop(&rect(3001, 1000)).is_err());
}

#[test]
fn 邊距不可為負且必須小於短邊一半() {
    let mut h = rect(1000, 600);
    h.margin = -1;
    assert!(validate_hoop(&h).is_err());
    h.margin = 300; // 等於短邊的一半 → 安全範圍消失
    assert!(validate_hoop(&h).is_err());
    h.margin = 299;
    assert!(validate_hoop(&h).is_ok());
}

#[test]
fn 圓形的寬必須等於高() {
    let mut h = rect(1000, 800);
    h.shape = HoopShape::Circle;
    assert!(validate_hoop(&h).is_err());
    h.height = 1000;
    assert!(validate_hoop(&h).is_ok());
}

#[test]
fn 錯誤訊息是繁體中文白話() {
    let m = validate_hoop(&rect(0, 1000)).unwrap_err();
    assert!(!m.text.is_empty());
    assert!(m
        .text
        .chars()
        .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)));
}

#[test]
fn 方形安全範圍判斷() {
    let h = rect(1000, 600); // 安全範圍半寬 450、半高 250
    assert!(h.safe_contains(Point::new(0, 0)));
    assert!(h.safe_contains(Point::new(450, 250)));
    assert!(h.safe_contains(Point::new(-450, -250)));
    assert!(!h.safe_contains(Point::new(451, 0)));
    assert!(!h.safe_contains(Point::new(0, -251)));
}

#[test]
fn 圓形安全範圍判斷() {
    let mut h = rect(1000, 1000);
    h.shape = HoopShape::Circle; // 半徑 450
    assert!(h.safe_contains(Point::new(450, 0)));
    assert!(h.safe_contains(Point::new(0, -450)));
    assert!(!h.safe_contains(Point::new(451, 0)));
    assert!(!h.safe_contains(Point::new(330, 330))); // 距離約 466.7 > 450
    assert!(h.safe_contains(Point::new(318, 318))); // 距離約 449.7
}

#[test]
fn 橢圓安全範圍判斷() {
    let mut h = rect(1000, 600);
    h.shape = HoopShape::Ellipse; // 半軸 450 與 250
    assert!(h.safe_contains(Point::new(450, 0)));
    assert!(h.safe_contains(Point::new(0, 250)));
    assert!(!h.safe_contains(Point::new(450, 250)));
    assert!(!h.safe_contains(Point::new(0, 251)));
}

proptest! {
    #[test]
    fn 範圍內外判斷與公式一致(x in -2000i32..2000, y in -2000i32..2000) {
        let h = rect(1000, 600);
        let inside = x.abs() <= 450 && y.abs() <= 250;
        prop_assert_eq!(h.safe_contains(Point::new(x, y)), inside);
    }
}
