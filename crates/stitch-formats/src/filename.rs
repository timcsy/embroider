//! 輸出檔名規則（保守版，因為繡機的檔名限制未知）。
//! 主檔名只用 A-Z 與 0-9，最長 8 字元：設計名稱的前四個英數字加四位序號。
//! 見 specs/001-embroidery-text-tool/contracts/output-files.md。

/// 產生主檔名（不含副檔名）。沒有可用英數字時前綴為 `EMB`。
pub fn file_stem(design_name: &str, sequence: u32) -> String {
    let prefix: String = design_name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .take(4)
        .collect();
    let prefix = if prefix.is_empty() {
        "EMB".to_string()
    } else {
        prefix
    };
    format!("{prefix}{:04}", sequence % 10_000)
}

/// 主檔名加大寫副檔名。
pub fn file_name(stem: &str, extension: &str) -> String {
    format!("{stem}.{}", extension.to_ascii_uppercase())
}
