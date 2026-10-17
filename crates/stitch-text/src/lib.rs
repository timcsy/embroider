//! 文字轉多邊形：字型載入、排版（單行、自動換行）、字形輪廓轉折線環。
//! 輸出多邊形的單位是 0.1mm，y 軸向下，幾何中心在原點。

mod font;
mod layout;

pub use font::{FontInfo, FontSet};
pub use layout::{text_to_polygon, Layout, TextLayout};
mod pipeline;

pub use pipeline::{build_text_preview, LayoutSpec, TextRequest};
