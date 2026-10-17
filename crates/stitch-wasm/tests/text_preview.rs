use std::path::PathBuf;
use stitch_core::hoop::builtin_hoops;
use stitch_core::{Stitch, ThreadColor};
use stitch_formats::Registry;
use stitch_text::{FontSet, LayoutSpec, TextRequest};
use stitch_wasm::api;

fn fonts() -> FontSet {
    let mut set = FontSet::default();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fonts/NotoSansTC-subset.ttf");
    set.add("sans", "黑體", std::fs::read(path).unwrap())
        .unwrap();
    set
}

fn hoop(id: &str) -> stitch_core::hoop::Hoop {
    builtin_hoops().into_iter().find(|h| h.id == id).unwrap()
}

fn request(text: &str) -> TextRequest {
    TextRequest {
        text: text.into(),
        font_id: "sans".into(),
        height: 150.0,
        color: ThreadColor::new(200, 30, 30, "正紅"),
        layout: LayoutSpec::SingleLine,
        hoop: hoop("b70-medium"),
        settings: Default::default(),
    }
}

#[test]
fn 字型清單() {
    let list = api::list_fonts(&fonts());
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, "sans");
}

#[test]
fn 預覽成功時所有針點在安全範圍內() {
    let r = api::build_text_preview(&fonts(), &request("Amy 2026"));
    assert!(r.blocked.is_none());
    let h = hoop("b70-medium");
    assert!(r.plan.stitches.iter().all(|s| h.safe_contains(s.point())));
}

#[test]
fn 全部格式一起輸出時檔名共用主檔名() {
    let r = api::build_text_preview(&fonts(), &request("Amy 2026"));
    let files = api::encode_all(&r.plan, &hoop("b70-medium"), &[], "Amy 2026", 1).unwrap();
    let names: Vec<_> = files.iter().map(|f| f.file_name.as_str()).collect();
    assert_eq!(names, vec!["AMY20001.DST", "AMY20001.EXP"]);
    assert!(files.iter().all(|f| !f.bytes.is_empty()));
}

#[test]
fn 指定格式時只輸出該格式() {
    let r = api::build_text_preview(&fonts(), &request("Amy"));
    let files =
        api::encode_all(&r.plan, &hoop("b70-medium"), &["exp".to_string()], "Amy", 3).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_name, "AMY0003.EXP");
}

#[test]
fn 各格式讀回後範圍與換色數一致() {
    let r = api::build_text_preview(&fonts(), &request("王小明"));
    let files = api::encode_all(&r.plan, &hoop("b70-medium"), &[], "x", 1).unwrap();
    let registry = Registry::default();
    let bounds = |plan: &stitch_core::StitchPlan| {
        plan.stitches
            .iter()
            .fold((i32::MAX, i32::MIN, i32::MAX, i32::MIN), |a, s| {
                (a.0.min(s.x), a.1.max(s.x), a.2.min(s.y), a.3.max(s.y))
            })
    };
    let original = bounds(&r.plan);
    for (f, id) in files.iter().zip(["dst", "exp"]) {
        let back = registry.get(id).unwrap().decode(&f.bytes).unwrap();
        assert_eq!(bounds(&back), original, "{id}");
        assert_eq!(
            back.color_change_count(),
            r.plan.color_change_count(),
            "{id}"
        );
    }
}

#[test]
fn 超出框的計畫無法輸出() {
    let mut r = api::build_text_preview(&fonts(), &request("Amy"));
    r.plan.stitches.insert(0, Stitch::stitch(5000, 0));
    let e = api::encode_all(&r.plan, &hoop("b70-medium"), &[], "Amy", 1).unwrap_err();
    assert_eq!(e.code, "out_of_hoop");
    let e = api::encode(&r.plan, &hoop("b70-medium"), "dst", "Amy", 1).unwrap_err();
    assert_eq!(e.code, "out_of_hoop");
}

#[test]
fn 不認得的格式回報白話錯誤() {
    let r = api::build_text_preview(&fonts(), &request("Amy"));
    let e = api::encode(&r.plan, &hoop("b70-medium"), "nope", "Amy", 1).unwrap_err();
    assert_eq!(e.code, "unknown_format");
    let e = api::encode_all(
        &r.plan,
        &hoop("b70-medium"),
        &["nope".to_string()],
        "Amy",
        1,
    )
    .unwrap_err();
    assert_eq!(e.code, "unknown_format");
}

#[test]
fn 被阻止的預覽沒有可輸出的針跡() {
    let r = api::build_text_preview(&fonts(), &request("  "));
    assert!(r.blocked.is_some());
    let e = api::encode_all(&r.plan, &hoop("b70-medium"), &[], "x", 1).unwrap_err();
    assert_eq!(e.code, "empty_plan");
}
