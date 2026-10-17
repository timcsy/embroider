//! 刺繡用的 2D 幾何：多邊形（非零填充）、曲線展平、掃描線求交、偏移。
//!
//! 這裡的座標是浮點數（單位 0.1mm，與核心一致），最後才由針跡生成取整。

mod flatten;
mod offset;
mod polygon;
mod scanline;

pub use flatten::flatten_path;
pub use polygon::{Polygon, Pt, Ring};
pub use scanline::spans_at;
