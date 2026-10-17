use crate::hoop::Hoop;
use crate::{StitchKind, StitchPlan, UserMessage};

/// 檢查針跡計畫是否可以安全輸出：
/// 非空、以結束針收尾、換色數與顏色數一致、全部針點在安全範圍內。
pub fn validate_plan(plan: &StitchPlan, hoop: &Hoop) -> Result<(), UserMessage> {
    if plan.stitches.is_empty() {
        return Err(UserMessage::new(
            "empty_plan",
            "還沒有可以繡的內容。",
            Some("請先輸入文字或選一張圖。"),
        ));
    }
    if plan.stitches.last().map(|s| s.kind) != Some(StitchKind::End) {
        return Err(UserMessage::new(
            "missing_end",
            "針跡沒有正確結束。",
            Some("請重新產生一次。"),
        ));
    }
    if plan.colors.len() != plan.color_change_count() + 1 {
        return Err(UserMessage::new(
            "color_mismatch",
            "線的顏色數量和換線次數對不起來。",
            Some("請重新產生一次。"),
        ));
    }
    if plan.stitches.iter().any(|s| !hoop.safe_contains(s.point())) {
        return Err(UserMessage::new(
            "out_of_hoop",
            "這個設計超出了框可以繡的範圍，繡的時候針可能會打到框。",
            Some("請把設計縮小一點，或換一個比較大的框。"),
        ));
    }
    Ok(())
}
