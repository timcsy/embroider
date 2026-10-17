use serde::{Deserialize, Serialize};

/// 給使用者看的訊息。`text` 是白話原因，`hint` 是建議的下一步。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserMessage {
    pub code: String,
    pub text: String,
    pub hint: Option<String>,
}

impl UserMessage {
    pub fn new(code: &str, text: &str, hint: Option<&str>) -> Self {
        Self {
            code: code.to_string(),
            text: text.to_string(),
            hint: hint.map(str::to_string),
        }
    }
}
