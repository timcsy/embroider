use stitch_core::hoop::{Hoop, HoopShape};
use stitch_core::{validate_plan, Stitch, StitchPlan, ThreadColor};

fn hoop() -> Hoop {
    Hoop {
        id: "t".into(),
        name: "測試框".into(),
        width: 1000,
        height: 600,
        shape: HoopShape::Rect,
        margin: 50,
        builtin: false,
    }
}

fn black() -> ThreadColor {
    ThreadColor::new(0, 0, 0, "黑")
}

fn good_plan() -> StitchPlan {
    StitchPlan {
        stitches: vec![
            Stitch::stitch(0, 0),
            Stitch::stitch(100, 100),
            Stitch::end(100, 100),
        ],
        colors: vec![black()],
    }
}

#[test]
fn 合法的計畫通過() {
    assert!(validate_plan(&good_plan(), &hoop()).is_ok());
}

#[test]
fn 空計畫被拒絕() {
    let plan = StitchPlan {
        stitches: vec![],
        colors: vec![],
    };
    let m = validate_plan(&plan, &hoop()).unwrap_err();
    assert_eq!(m.code, "empty_plan");
}

#[test]
fn 沒有結束針被拒絕() {
    let mut plan = good_plan();
    plan.stitches.pop();
    assert_eq!(
        validate_plan(&plan, &hoop()).unwrap_err().code,
        "missing_end"
    );
}

#[test]
fn 超出安全範圍被拒絕並用白話說明() {
    let mut plan = good_plan();
    plan.stitches.insert(1, Stitch::stitch(451, 0));
    let m = validate_plan(&plan, &hoop()).unwrap_err();
    assert_eq!(m.code, "out_of_hoop");
    assert!(m.hint.is_some());
}

#[test]
fn 換色次數必須與顏色數一致() {
    let mut plan = good_plan();
    plan.stitches.insert(2, Stitch::color_change(100, 100));
    // 一次換色需要兩種顏色，但只有一種
    assert_eq!(
        validate_plan(&plan, &hoop()).unwrap_err().code,
        "color_mismatch"
    );
    plan.colors.push(ThreadColor::new(255, 0, 0, "紅"));
    assert!(validate_plan(&plan, &hoop()).is_ok());
}
