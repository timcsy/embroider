use stitch_core::StitchPlan;
use stitch_formats::{Format, FormatError, Registry};

struct Fake;
impl Format for Fake {
    fn id(&self) -> &'static str {
        "fake"
    }
    fn extension(&self) -> &'static str {
        "FAK"
    }
    fn label(&self) -> &'static str {
        "測試格式"
    }
    fn description(&self) -> &'static str {
        "只給測試用"
    }
    fn max_step(&self) -> i32 {
        10
    }
    fn encode(&self, _plan: &StitchPlan, _label: &str) -> Result<Vec<u8>, FormatError> {
        Ok(vec![1, 2, 3])
    }
    fn decode(&self, _bytes: &[u8]) -> Result<StitchPlan, FormatError> {
        Ok(StitchPlan::default())
    }
}

fn sample_plan() -> StitchPlan {
    use stitch_core::{Stitch, ThreadColor};
    StitchPlan {
        stitches: vec![
            Stitch::stitch(10, 20),
            Stitch::stitch(30, 40),
            Stitch::end(30, 40),
        ],
        colors: vec![ThreadColor::new(0, 0, 0, "黑")],
    }
}

#[test]
fn 預設註冊_dst_與_exp() {
    let r = Registry::default();
    let ids: Vec<_> = r.list().iter().map(|f| f.id()).collect();
    assert_eq!(ids, vec!["dst", "exp"]);
    assert_eq!(r.get("dst").unwrap().extension(), "DST");
    assert_eq!(r.get("exp").unwrap().extension(), "EXP");
    assert!(r.get("nope").is_none());
}

#[test]
fn 每個格式都有給人看的名稱和說明() {
    let r = Registry::default();
    for f in r.list() {
        assert!(!f.label().is_empty());
        assert!(!f.description().is_empty());
        assert!(f.max_step() > 0);
    }
}

#[test]
fn 新增格式不影響既有格式的輸出() {
    let plan = sample_plan();
    let before: Vec<Vec<u8>> = Registry::default()
        .list()
        .iter()
        .map(|f| f.encode(&plan, "T").unwrap())
        .collect();

    let mut r = Registry::default();
    r.register(Box::new(Fake));
    assert_eq!(r.list().len(), 3);
    assert_eq!(
        r.get("fake").unwrap().encode(&plan, "T").unwrap(),
        vec![1, 2, 3]
    );

    let after: Vec<Vec<u8>> = r
        .list()
        .iter()
        .filter(|f| f.id() != "fake")
        .map(|f| f.encode(&plan, "T").unwrap())
        .collect();
    assert_eq!(before, after);
}
