use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// 針點類型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StitchKind {
    /// 一般針
    Stitch,
    /// 跳針（不下針移動）
    Jump,
    /// 剪線
    Trim,
    /// 換色
    ColorChange,
    /// 結束
    End,
}

/// 絕對座標的針點。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stitch {
    pub x: i32,
    pub y: i32,
    pub kind: StitchKind,
}

#[allow(clippy::self_named_constructors)]
impl Stitch {
    fn with(x: i32, y: i32, kind: StitchKind) -> Self {
        Self { x, y, kind }
    }
    pub fn stitch(x: i32, y: i32) -> Self {
        Self::with(x, y, StitchKind::Stitch)
    }
    pub fn jump(x: i32, y: i32) -> Self {
        Self::with(x, y, StitchKind::Jump)
    }
    pub fn trim(x: i32, y: i32) -> Self {
        Self::with(x, y, StitchKind::Trim)
    }
    pub fn color_change(x: i32, y: i32) -> Self {
        Self::with(x, y, StitchKind::ColorChange)
    }
    pub fn end(x: i32, y: i32) -> Self {
        Self::with(x, y, StitchKind::End)
    }
    pub fn point(&self) -> Point {
        Point::new(self.x, self.y)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// 給使用者看的中文色名
    pub name: String,
}

impl ThreadColor {
    pub fn new(r: u8, g: u8, b: u8, name: &str) -> Self {
        Self {
            r,
            g,
            b,
            name: name.to_string(),
        }
    }
}

/// 最終要送給機器的針跡序列。`colors` 依換色順序排列，
/// 因此非空計畫的顏色數 = 換色次數 + 1。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StitchPlan {
    pub stitches: Vec<Stitch>,
    pub colors: Vec<ThreadColor>,
}

impl StitchPlan {
    pub fn color_change_count(&self) -> usize {
        self.stitches
            .iter()
            .filter(|s| s.kind == StitchKind::ColorChange)
            .count()
    }
    pub fn stitch_count(&self) -> usize {
        self.stitches
            .iter()
            .filter(|s| s.kind == StitchKind::Stitch)
            .count()
    }
}
