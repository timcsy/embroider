//! 針跡生成：填充（tatami）、輪廓、底針、固定針與跳針剪線、最小尺寸與縮放適配。
//! 全部是純函式：相同輸入與參數得到相同輸出。座標為浮點數，單位 0.1mm。

pub mod design;
pub mod fill;
pub mod fit;
pub mod min_size;
pub mod outline;
pub mod travel;
pub mod underlay;

/// 一段連續下針的點序列。
pub type Run = Vec<[f64; 2]>;
