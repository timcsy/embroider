use proptest::prelude::*;
use stitch_formats::filename::{file_name, file_stem};

#[test]
fn 英文名稱取前四個英數字轉大寫加四位序號() {
    assert_eq!(file_stem("Amy", 1), "AMY0001");
    assert_eq!(file_stem("amy lee", 12), "AMYL0012");
}

#[test]
fn 中文與符號略過_只取英數字() {
    assert_eq!(file_stem("王小明 2026", 1), "20260001");
    assert_eq!(file_stem("A-B_C!D@E", 3), "ABCD0003");
}

#[test]
fn 沒有可用字元時前綴是_emb() {
    assert_eq!(file_stem("王小明", 1), "EMB0001");
    assert_eq!(file_stem("", 7), "EMB0007");
    assert_eq!(file_stem("   ", 7), "EMB0007");
    assert_eq!(file_stem("！？", 7), "EMB0007");
}

#[test]
fn 序號超過九千九百九十九會從頭算() {
    assert_eq!(file_stem("Amy", 10000), "AMY0000");
    assert_eq!(file_stem("Amy", 10001), "AMY0001");
}

#[test]
fn 檔名是主檔名加大寫副檔名() {
    assert_eq!(file_name("AMY0001", "DST"), "AMY0001.DST");
    assert_eq!(file_name("AMY0001", "exp"), "AMY0001.EXP");
}

#[test]
fn 同一次輸出共用主檔名() {
    let stem = file_stem("Amy", 5);
    assert_eq!(file_name(&stem, "DST"), "AMY0005.DST");
    assert_eq!(file_name(&stem, "EXP"), "AMY0005.EXP");
}

proptest! {
    #[test]
    fn 任意名稱與序號都產生合法主檔名(name in ".{0,60}", seq in 0u32..100_000) {
        let stem = file_stem(&name, seq);
        prop_assert!(!stem.is_empty() && stem.len() <= 8, "{stem}");
        prop_assert!(stem.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()), "{stem}");
    }
}
