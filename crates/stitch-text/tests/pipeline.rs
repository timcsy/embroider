use std::path::PathBuf;
use stitch_core::hoop::{builtin_hoops, Hoop};
use stitch_core::{validate_plan, StitchKind, ThreadColor};
use stitch_text::{build_text_preview, FontSet, LayoutSpec, TextRequest};

fn fonts() -> FontSet {
    let mut set = FontSet::default();
    for (id, name, file) in [
        ("sans", "黑體", "NotoSansTC-subset.ttf"),
        ("serif", "宋體", "NotoSerifTC-subset.ttf"),
    ] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fonts")
            .join(file);
        set.add(id, name, std::fs::read(path).unwrap()).unwrap();
    }
    set
}

fn hoop(id: &str) -> Hoop {
    builtin_hoops().into_iter().find(|h| h.id == id).unwrap()
}

fn req(text: &str, height: f64, hoop_id: &str) -> TextRequest {
    TextRequest {
        text: text.to_string(),
        font_id: "sans".into(),
        height,
        color: ThreadColor::new(200, 30, 30, "正紅"),
        layout: LayoutSpec::SingleLine,
        hoop: hoop(hoop_id),
        settings: Default::default(),
    }
}

#[test]
fn 一般文字產生合法的計畫() {
    let r = build_text_preview(&fonts(), &req("王小明 2026", 150.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert_eq!(r.fitted_scale, 1.0);
    assert!(
        r.plan.stitch_count() > 1000,
        "針數 {}",
        r.plan.stitch_count()
    );
    assert_eq!(r.plan.colors.len(), 1);
    validate_plan(&r.plan, &hoop("b70-medium")).unwrap();
}

#[test]
fn 空白文字被阻止並說明() {
    for t in ["", "  "] {
        let r = build_text_preview(&fonts(), &req(t, 150.0, "b70-medium"));
        assert_eq!(r.blocked.unwrap().code, "empty_text");
        assert!(r.plan.stitches.is_empty());
    }
}

#[test]
fn 字太小會放大() {
    let r = build_text_preview(&fonts(), &req("王小明", 30.0, "b70-medium"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert!(r.adjustments.iter().any(|m| m.code == "enlarged"));
}

#[test]
fn 字太大會縮小到放進小框() {
    let r = build_text_preview(&fonts(), &req("王小明", 300.0, "b70-small"));
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    assert!(r.fitted_scale < 1.0);
    assert!(r.adjustments.iter().any(|m| m.code == "shrunk"));
    validate_plan(&r.plan, &hoop("b70-small")).unwrap();
}

#[test]
fn 太長的文字放不進小框時被阻止() {
    let r = build_text_preview(
        &fonts(),
        &req("王小明的刺繡作品展覽很長的一段文字", 100.0, "b70-small"),
    );
    assert_eq!(r.blocked.unwrap().code, "too_big_for_hoop");
}

#[test]
fn 字型缺字會說明並略過() {
    let r = build_text_preview(&fonts(), &req("A龘B", 150.0, "b70-medium"));
    assert!(r.blocked.is_none());
    let m = r
        .adjustments
        .iter()
        .find(|m| m.code == "missing_glyphs")
        .expect("應說明缺字");
    assert!(m.text.contains('龘'));
}

#[test]
fn 全部缺字時被阻止() {
    let r = build_text_preview(&fonts(), &req("龘", 150.0, "b70-medium"));
    assert_eq!(r.blocked.unwrap().code, "no_glyphs");
}

#[test]
fn 找不到字型時被阻止() {
    let mut q = req("A", 150.0, "b70-medium");
    q.font_id = "nope".into();
    assert_eq!(
        build_text_preview(&fonts(), &q).blocked.unwrap().code,
        "unknown_font"
    );
}

#[test]
fn 單一字母不會到處亂跳針() {
    let r = build_text_preview(&fonts(), &req("H", 200.0, "b70-medium"));
    assert!(r.blocked.is_none());
    let jumps = r
        .plan
        .stitches
        .iter()
        .filter(|s| s.kind == StitchKind::Jump)
        .count();
    // H 有底針外圈、底針格網 3 條、面針 3 條、邊緣輪廓，至少 8 組之間的轉換；實測 15 次。
    // 若逐列來回縫（沒有分條），會是上百次。
    assert!(jumps <= 20, "跳針 {jumps} 次");
}

#[test]
fn 相同輸入得到相同輸出() {
    let a = build_text_preview(&fonts(), &req("Amy 2026", 150.0, "b70-medium"));
    let b = build_text_preview(&fonts(), &req("Amy 2026", 150.0, "b70-medium"));
    assert_eq!(a, b);
}

#[test]
fn 沒有被阻止時一律在安全範圍內() {
    let texts = [
        "A",
        "王小明",
        "Happy Birthday",
        "祝你生日快樂",
        "2026-10-17",
        "媽媽",
    ];
    for hoop_id in ["b70-small", "b70-medium", "b70-large"] {
        for t in texts {
            for h in [30.0, 100.0, 250.0] {
                let r = build_text_preview(&fonts(), &req(t, h, hoop_id));
                if r.blocked.is_none() {
                    validate_plan(&r.plan, &hoop(hoop_id))
                        .unwrap_or_else(|m| panic!("{t} {h} {hoop_id}: {m:?}"));
                }
            }
        }
    }
}

#[test]
fn 自動換行時寬度不超過限制() {
    let mut q = req("王小明的刺繡作品展覽", 100.0, "b70-medium");
    q.layout = LayoutSpec::Wrap { max_width: 500.0 };
    let r = build_text_preview(&fonts(), &q);
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    let (min_x, max_x) = r
        .plan
        .stitches
        .iter()
        .fold((i32::MAX, i32::MIN), |a, s| (a.0.min(s.x), a.1.max(s.x)));
    assert!(max_x - min_x <= 500 + 10, "寬 {}", max_x - min_x);
}

fn plan_size(r: &stitch_gen::design::PreviewResult) -> (i32, i32) {
    let (min_x, max_x, min_y, max_y) = r
        .plan
        .stitches
        .iter()
        .fold((i32::MAX, i32::MIN, i32::MAX, i32::MIN), |a, s| {
            (a.0.min(s.x), a.1.max(s.x), a.2.min(s.y), a.3.max(s.y))
        });
    (max_x - min_x, max_y - min_y)
}

#[test]
fn 長句子單行放不下時自動分成幾行() {
    let r = build_text_preview(
        &fonts(),
        &req("祝你生日快樂身體健康萬事如意心想事成", 250.0, "b70-large"),
    );
    assert!(r.blocked.is_none(), "{:?}", r.blocked);
    let m = r
        .adjustments
        .iter()
        .find(|m| m.code == "wrapped")
        .expect("應說明已分行");
    assert!(m.text.contains("行"));
    let (w, h) = plan_size(&r);
    assert!(h > 250, "多行時高度應超過單行字高：{h}");
    assert!(w <= 1500, "寬度不超過大框安全寬：{w}");
    validate_plan(&r.plan, &hoop("b70-large")).unwrap();
}

#[test]
fn 名字這種短文字不會被分行() {
    for t in ["王小明", "王小明的刺繡作"] {
        let r = build_text_preview(&fonts(), &req(t, 250.0, "b70-large"));
        assert!(r.blocked.is_none(), "{t}: {:?}", r.blocked);
        assert!(
            r.adjustments.iter().all(|m| m.code != "wrapped"),
            "{t} 不該分行"
        );
    }
}

#[test]
fn 只要稍微縮小就放得下時維持單行() {
    // 七個字在大框只需縮到約 85%，不值得拆行
    let r = build_text_preview(&fonts(), &req("王小明的刺繡作", 250.0, "b70-large"));
    assert!(
        r.fitted_scale > 0.75 && r.fitted_scale < 1.0,
        "{}",
        r.fitted_scale
    );
    let (_, h) = plan_size(&r);
    assert!(h < 250 * 2, "單行高度：{h}");
}

#[test]
fn 分行後仍然放不下時照樣阻止並說明() {
    // 小框很窄，再怎麼分行也放不下這麼多字
    let long = "祝你生日快樂身體健康萬事如意心想事成".repeat(3);
    let r = build_text_preview(&fonts(), &req(&long, 250.0, "b70-small"));
    assert_eq!(r.blocked.unwrap().code, "too_big_for_hoop");
}

#[test]
fn 使用者指定換行寬度時尊重它而不另外分行() {
    let mut q = req("祝你生日快樂身體健康", 250.0, "b70-large");
    q.layout = LayoutSpec::Wrap { max_width: 600.0 };
    let r = build_text_preview(&fonts(), &q);
    assert!(r.blocked.is_none());
    assert!(r.adjustments.iter().all(|m| m.code != "wrapped"));
}

#[test]
fn 手動換行字元一律尊重() {
    let r = build_text_preview(&fonts(), &req("王小明\n生日快樂", 250.0, "b70-large"));
    assert!(r.blocked.is_none());
    let (_, h) = plan_size(&r);
    assert!(h > 250);
}
