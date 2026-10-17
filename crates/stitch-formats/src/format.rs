use std::fmt;
use stitch_core::StitchPlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// 資料太短或結構不對
    Malformed(String),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::Malformed(m) => write!(f, "檔案內容不正確：{m}"),
        }
    }
}

impl std::error::Error for FormatError {}

/// 一種刺繡檔案格式。
pub trait Format: Send + Sync {
    fn id(&self) -> &'static str;
    /// 大寫副檔名，不含點
    fn extension(&self) -> &'static str;
    /// 給使用者看的易懂名稱
    fn label(&self) -> &'static str;
    fn description(&self) -> &'static str;
    /// 單針最大位移（0.1mm）
    fn max_step(&self) -> i32;
    fn encode(&self, plan: &StitchPlan, label: &str) -> Result<Vec<u8>, FormatError>;
    /// 僅供測試與驗證往返使用
    fn decode(&self, bytes: &[u8]) -> Result<StitchPlan, FormatError>;
}

/// 格式註冊表。
pub struct Registry {
    formats: Vec<Box<dyn Format>>,
}

impl Registry {
    pub fn empty() -> Self {
        Self {
            formats: Vec::new(),
        }
    }
    pub fn register(&mut self, f: Box<dyn Format>) {
        self.formats.push(f);
    }
    pub fn list(&self) -> Vec<&dyn Format> {
        self.formats.iter().map(|f| f.as_ref()).collect()
    }
    pub fn get(&self, id: &str) -> Option<&dyn Format> {
        self.formats
            .iter()
            .find(|f| f.id() == id)
            .map(|f| f.as_ref())
    }
}

impl Default for Registry {
    fn default() -> Self {
        let mut r = Registry::empty();
        r.register(Box::new(crate::dst::Dst));
        r.register(Box::new(crate::exp::Exp));
        r
    }
}

/// 解碼後沒有顏色資訊的格式，用佔位顏色補足（顏色數 = 換色數 + 1）。
pub(crate) fn placeholder_colors(changes: usize) -> Vec<stitch_core::ThreadColor> {
    (0..=changes)
        .map(|_| stitch_core::ThreadColor::new(128, 128, 128, "未知"))
        .collect()
}
