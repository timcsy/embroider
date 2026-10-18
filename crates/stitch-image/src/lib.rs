//! 圖片轉刺繡：色彩分群、逐色轉多邊形、以實際尺寸做平滑與合併過細的部分。

pub mod color;
mod pipeline;
pub mod quantize;
pub mod saliency;
pub mod smooth;
pub mod trace;

pub use pipeline::{build_image_preview, ImageRequest};
