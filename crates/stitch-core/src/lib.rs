//! 刺繡核心的基礎型別、繡框模型與計畫驗證。
//!
//! 所有長度單位為 0.1mm 的整數（例如 160mm = 1600）。

pub mod hoop;
pub mod message;
pub mod palette;
pub mod types;
pub mod validate;

pub use message::UserMessage;
pub use types::{Point, Stitch, StitchKind, StitchPlan, ThreadColor};
pub use validate::validate_plan;
