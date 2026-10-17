use std::path::PathBuf;
use stitch_geom::Polygon;
use stitch_text::{text_to_polygon, FontSet, Layout};

fn font_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fonts")
        .join(name)
}

fn fonts() -> FontSet {
    let mut set = FontSet::default();
    for (id, name, file) in [
        ("sans", "黑體", "NotoSansTC-subset.ttf"),
        ("serif", "宋體", "NotoSerifTC-subset.ttf"),
    ] {
        set.add(id, name, std::fs::read(font_path(file)).unwrap())
            .unwrap();
    }
    set
}

fn size(p: &Polygon) -> (f64, f64) {
    let (min, max) = p.bounds().unwrap();
    (max[0] - min[0], max[1] - min[1])
}

#[test]
fn 字型清單與壞資料() {
    let mut set = fonts();
    let ids: Vec<_> = set.list().iter().map(|f| f.id.clone()).collect();
    assert_eq!(ids, vec!["sans", "serif"]);
    assert!(set.add("bad", "壞的", vec![1, 2, 3]).is_err());
    assert_eq!(set.list().len(), 2);
}

#[test]
fn 英文字母_a_有面積且不超過字級() {
    let r = text_to_polygon(&fonts(), "sans", "A", 100.0, Layout::SingleLine).unwrap();
    let (w, h) = size(&r.polygon);
    assert!(r.polygon.area() > 100.0);
    assert!(w > 30.0 && w < 100.0, "w={w}");
    assert!(h > 50.0 && h <= 100.0, "h={h}");
    assert!(r.missing.is_empty());
}

#[test]
fn 數字_8_有兩個洞() {
    let r = text_to_polygon(&fonts(), "sans", "8", 100.0, Layout::SingleLine).unwrap();
    assert_eq!(r.polygon.normalized().rings.len(), 3);
}

#[test]
fn 中文字大小合理_筆畫多的字面積較大() {
    let one = text_to_polygon(&fonts(), "sans", "一", 100.0, Layout::SingleLine).unwrap();
    let dragon = text_to_polygon(&fonts(), "sans", "龍", 100.0, Layout::SingleLine).unwrap();
    let king = text_to_polygon(&fonts(), "sans", "王", 100.0, Layout::SingleLine).unwrap();
    // 實測：一 ≈ 957、龍 ≈ 4491（字級 100）；筆畫多的字面積應明顯較大
    assert!(dragon.polygon.area() > one.polygon.area() * 3.0);
    let (w, h) = size(&king.polygon);
    assert!(
        w > 60.0 && w <= 100.0 && h > 60.0 && h <= 100.0,
        "w={w} h={h}"
    );
}

#[test]
fn 單行排版置中在原點且寬度隨字數增加() {
    let a = text_to_polygon(&fonts(), "sans", "AB", 100.0, Layout::SingleLine).unwrap();
    let b = text_to_polygon(&fonts(), "sans", "ABAB", 100.0, Layout::SingleLine).unwrap();
    let (wa, _) = size(&a.polygon);
    let (wb, _) = size(&b.polygon);
    assert!(wb > wa * 1.8, "wa={wa} wb={wb}");
    let (min, max) = b.polygon.bounds().unwrap();
    assert!(((min[0] + max[0]) / 2.0).abs() < 1.0);
    assert!(((min[1] + max[1]) / 2.0).abs() < 1.0);
}

#[test]
fn 自動換行後寬度不超過限制且高度增加() {
    let text = "王小明的刺繡作品展覽";
    let single = text_to_polygon(&fonts(), "sans", text, 100.0, Layout::SingleLine).unwrap();
    let wrapped = text_to_polygon(
        &fonts(),
        "sans",
        text,
        100.0,
        Layout::Wrap { max_width: 400.0 },
    )
    .unwrap();
    let (ws, hs) = size(&single.polygon);
    let (ww, hw) = size(&wrapped.polygon);
    assert!(ws > 400.0);
    assert!(ww <= 400.0 + 1.0, "ww={ww}");
    assert!(hw > hs * 1.5, "hs={hs} hw={hw}");
}

#[test]
fn 手動換行字元也會換行() {
    let r = text_to_polygon(&fonts(), "sans", "A\nB", 100.0, Layout::SingleLine).unwrap();
    let (_, h) = size(&r.polygon);
    assert!(h > 100.0);
}

#[test]
fn 字型沒有的字會回報而其他字照常排() {
    let r = text_to_polygon(&fonts(), "sans", "A龘B", 100.0, Layout::SingleLine).unwrap();
    assert_eq!(r.missing, vec!['龘']);
    assert!(r.polygon.area() > 100.0);
}

#[test]
fn 全部都缺字時回報白話錯誤() {
    let e = text_to_polygon(&fonts(), "sans", "龘", 100.0, Layout::SingleLine).unwrap_err();
    assert_eq!(e.code, "no_glyphs");
    assert!(e.hint.is_some());
}

#[test]
fn 空白文字回報白話錯誤() {
    for t in ["", "   ", "\n \n"] {
        let e = text_to_polygon(&fonts(), "sans", t, 100.0, Layout::SingleLine).unwrap_err();
        assert_eq!(e.code, "empty_text", "輸入 {t:?}");
    }
}

#[test]
fn 找不到字型時回報錯誤() {
    let e = text_to_polygon(&fonts(), "nope", "A", 100.0, Layout::SingleLine).unwrap_err();
    assert_eq!(e.code, "unknown_font");
}

#[test]
fn 相同輸入得到相同輸出() {
    let a = text_to_polygon(&fonts(), "serif", "王小明 2026", 120.0, Layout::SingleLine).unwrap();
    let b = text_to_polygon(&fonts(), "serif", "王小明 2026", 120.0, Layout::SingleLine).unwrap();
    assert_eq!(a.polygon, b.polygon);
}

#[test]
fn 不同字型的字形不同() {
    let a = text_to_polygon(&fonts(), "sans", "永", 100.0, Layout::SingleLine).unwrap();
    let b = text_to_polygon(&fonts(), "serif", "永", 100.0, Layout::SingleLine).unwrap();
    assert_ne!(a.polygon, b.polygon);
}
