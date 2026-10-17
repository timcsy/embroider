use stitch_core::hoop::builtin_hoops;
use stitch_core::{validate_plan, Stitch, StitchKind, ThreadColor};
use stitch_gen::travel::{assemble, group_by_color, ColorBlock, TravelParams};

fn black() -> ThreadColor {
    ThreadColor::new(0, 0, 0, "黑")
}
fn red() -> ThreadColor {
    ThreadColor::new(200, 30, 30, "正紅")
}

fn kinds(plan: &stitch_core::StitchPlan) -> Vec<StitchKind> {
    plan.stitches.iter().map(|s| s.kind).collect()
}

#[test]
fn 單一線段的_golden_含固定針() {
    let blocks = vec![ColorBlock {
        color: black(),
        runs: vec![vec![[0.0, 0.0], [20.0, 0.0]]],
    }];
    let plan = assemble(&blocks, &TravelParams::default());
    assert_eq!(
        plan.stitches,
        vec![
            Stitch::stitch(4, 0),
            Stitch::stitch(0, 0),
            Stitch::stitch(20, 0),
            Stitch::stitch(16, 0),
            Stitch::stitch(20, 0),
            Stitch::end(20, 0),
        ]
    );
    assert_eq!(plan.colors.len(), 1);
}

#[test]
fn 第一針一定先跳針到起點() {
    let blocks = vec![ColorBlock {
        color: black(),
        runs: vec![vec![[50.0, 10.0], [70.0, 10.0]]],
    }];
    let plan = assemble(&blocks, &TravelParams::default());
    assert_eq!(plan.stitches[0], Stitch::jump(50, 10));
}

#[test]
fn 間距不超過跳針門檻就直接縫過去() {
    let blocks = vec![ColorBlock {
        color: black(),
        runs: vec![
            vec![[0.0, 0.0], [20.0, 0.0]],
            vec![[20.0, 20.0], [0.0, 20.0]],
        ],
    }];
    let plan = assemble(&blocks, &TravelParams::default());
    assert!(!kinds(&plan).contains(&StitchKind::Jump));
    assert!(!kinds(&plan).contains(&StitchKind::Trim));
}

#[test]
fn 超過跳針門檻用跳針_超過剪線門檻先剪線() {
    let p = TravelParams::default();
    let mid = vec![ColorBlock {
        color: black(),
        runs: vec![
            vec![[0.0, 0.0], [20.0, 0.0]],
            vec![[20.0, 50.0], [0.0, 50.0]],
        ],
    }];
    let k = kinds(&assemble(&mid, &p));
    assert!(k.contains(&StitchKind::Jump));
    assert!(!k.contains(&StitchKind::Trim));

    let far = vec![ColorBlock {
        color: black(),
        runs: vec![
            vec![[0.0, 0.0], [20.0, 0.0]],
            vec![[20.0, 120.0], [0.0, 120.0]],
        ],
    }];
    let plan = assemble(&far, &p);
    let k = kinds(&plan);
    let trim_at = k
        .iter()
        .position(|x| *x == StitchKind::Trim)
        .expect("應該剪線");
    assert_eq!(k[trim_at + 1], StitchKind::Jump, "剪線後緊接著跳針");
    // 跳針後有固定針（兩針很短的往返）
    assert_eq!(plan.stitches[trim_at + 2].kind, StitchKind::Stitch);
}

#[test]
fn 換色前先固定並在新位置跳針() {
    let blocks = vec![
        ColorBlock {
            color: black(),
            runs: vec![vec![[0.0, 0.0], [20.0, 0.0]]],
        },
        ColorBlock {
            color: red(),
            runs: vec![vec![[40.0, 40.0], [60.0, 40.0]]],
        },
    ];
    let plan = assemble(&blocks, &TravelParams::default());
    assert_eq!(plan.color_change_count(), 1);
    assert_eq!(plan.colors, vec![black(), red()]);
    let k = kinds(&plan);
    let c = k
        .iter()
        .position(|x| *x == StitchKind::ColorChange)
        .unwrap();
    assert_eq!(k[c + 1], StitchKind::Jump);
    assert_eq!(plan.stitches[c + 1], Stitch::jump(40, 40));
}

#[test]
fn 以結束針收尾且沒有連續重複的針點() {
    let blocks = vec![ColorBlock {
        color: black(),
        runs: vec![
            vec![[0.0, 0.0], [0.2, 0.0], [20.0, 0.0]],
            vec![[20.0, 10.0], [0.0, 10.0]],
        ],
    }];
    let plan = assemble(&blocks, &TravelParams::default());
    assert_eq!(plan.stitches.last().unwrap().kind, StitchKind::End);
    for w in plan.stitches.windows(2) {
        let same = w[0].x == w[1].x && w[0].y == w[1].y;
        assert!(!(same && w[0].kind == StitchKind::Stitch && w[1].kind == StitchKind::Stitch));
    }
}

#[test]
fn 組出的計畫通過安全範圍驗證() {
    let blocks = vec![
        ColorBlock {
            color: black(),
            runs: vec![vec![[0.0, 0.0], [20.0, 0.0]]],
        },
        ColorBlock {
            color: red(),
            runs: vec![vec![[40.0, 40.0], [60.0, 40.0]]],
        },
    ];
    let plan = assemble(&blocks, &TravelParams::default());
    let hoop = builtin_hoops()
        .into_iter()
        .find(|h| h.id == "b70-small")
        .unwrap();
    assert!(validate_plan(&plan, &hoop).is_ok());
}

#[test]
fn 空的輸入產生只有結束針的計畫() {
    let plan = assemble(&[], &TravelParams::default());
    assert_eq!(plan.stitches.len(), 1);
    assert_eq!(plan.stitches[0].kind, StitchKind::End);
}

#[test]
fn 相同顏色的區塊會合併以減少換色() {
    let blocks = vec![
        ColorBlock {
            color: black(),
            runs: vec![vec![[0.0, 0.0], [10.0, 0.0]]],
        },
        ColorBlock {
            color: red(),
            runs: vec![vec![[0.0, 20.0], [10.0, 20.0]]],
        },
        ColorBlock {
            color: black(),
            runs: vec![vec![[0.0, 40.0], [10.0, 40.0]]],
        },
    ];
    let grouped = group_by_color(blocks);
    assert_eq!(grouped.len(), 2);
    assert_eq!(grouped[0].color, black());
    assert_eq!(grouped[0].runs.len(), 2);
    assert_eq!(grouped[1].color, red());
}

#[test]
fn 區段開頭有極短的線段時固定針仍有足夠長度() {
    // 第二個點只差 0.3，固定針應朝向夠遠的點，而不是退化成原地
    let blocks = vec![ColorBlock {
        color: black(),
        runs: vec![vec![[0.0, 0.0], [0.3, 0.0], [30.0, 0.0]]],
    }];
    let plan = assemble(&blocks, &TravelParams::default());
    assert_eq!(plan.stitches[0], Stitch::stitch(4, 0));
    assert_eq!(plan.stitches[1], Stitch::stitch(0, 0));
}

#[test]
fn 區段結尾有極短的線段時固定針仍有足夠長度() {
    let blocks = vec![ColorBlock {
        color: black(),
        runs: vec![vec![[0.0, 0.0], [30.0, 0.0], [29.7, 0.0]]],
    }];
    let plan = assemble(&blocks, &TravelParams::default());
    let n = plan.stitches.len();
    // ... 終點 (30,0) → 固定針 (26,0) → 回到終點 → End
    assert_eq!(plan.stitches[n - 3], Stitch::stitch(26, 0));
}

#[test]
fn 計畫中不會有原地不動的針() {
    let blocks = vec![ColorBlock {
        color: black(),
        runs: vec![
            vec![[50.0, 10.0], [50.2, 10.0], [70.0, 10.0]],
            vec![[70.0, 10.0], [90.0, 10.0]],
            vec![[300.0, 10.0], [300.1, 10.0], [320.0, 10.0]],
        ],
    }];
    let plan = assemble(&blocks, &TravelParams::default());
    let mut prev: Option<(i32, i32)> = None;
    for s in &plan.stitches {
        if s.kind == StitchKind::Stitch {
            assert_ne!(Some((s.x, s.y)), prev, "原地不動的針：{s:?}");
        }
        prev = Some((s.x, s.y));
    }
}
