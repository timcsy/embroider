//! 核心對前端的介面（純 Rust、可原生測試）。`lib.rs` 的 `#[wasm_bindgen]` 函式只做型別轉接。
//! 契約見 specs/001-embroidery-text-tool/contracts/wasm-api.md。

use serde::{Deserialize, Serialize};
use stitch_core::hoop::{self, Hoop};
use stitch_core::{validate_plan, StitchPlan, UserMessage};
use stitch_formats::filename::{file_name, file_stem};
use stitch_formats::Registry;

pub use stitch_compose::{CompositeRequest, CompositeResult};
pub use stitch_gen::design::{PreviewResult, Settings};
pub use stitch_image::ImageRequest;
pub use stitch_text::{FontSet, LayoutSpec, TextRequest};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputFormatInfo {
    pub id: String,
    pub extension: String,
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoopValidation {
    pub ok: bool,
    pub message: Option<UserMessage>,
}

pub fn list_builtin_hoops() -> Vec<Hoop> {
    hoop::builtin_hoops()
}

pub fn list_formats() -> Vec<OutputFormatInfo> {
    Registry::default()
        .list()
        .iter()
        .map(|f| OutputFormatInfo {
            id: f.id().to_string(),
            extension: f.extension().to_string(),
            label: f.label().to_string(),
            description: f.description().to_string(),
        })
        .collect()
}

pub fn validate_hoop(h: &Hoop) -> HoopValidation {
    match hoop::validate_hoop(h) {
        Ok(()) => HoopValidation {
            ok: true,
            message: None,
        },
        Err(m) => HoopValidation {
            ok: false,
            message: Some(m),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontEntry {
    pub id: String,
    pub name: String,
}

pub fn list_fonts(fonts: &FontSet) -> Vec<FontEntry> {
    fonts
        .list()
        .into_iter()
        .map(|f| FontEntry {
            id: f.id,
            name: f.name,
        })
        .collect()
}

pub fn build_image_preview(req: &ImageRequest) -> PreviewResult {
    stitch_image::build_image_preview(req)
}

/// 依圖片內容建議的顏色數（2～8）。
pub fn suggest_color_count(rgba: &[u8], width: usize, height: usize) -> usize {
    stitch_image::quantize::suggest_color_count(rgba, width, height)
}

/// 線色色票（中文色名）。
pub fn list_thread_colors() -> Vec<stitch_core::ThreadColor> {
    stitch_core::palette::threads()
}

/// 多物件設計（文字、圖片、形狀）→ 針跡計畫與各物件的實際大小。
pub fn build_composite_preview(
    fonts: &FontSet,
    req: &CompositeRequest,
    images: &[Vec<u8>],
) -> CompositeResult {
    stitch_compose::build_composite(fonts, req, images)
}

pub fn build_text_preview(fonts: &FontSet, req: &TextRequest) -> PreviewResult {
    stitch_text::build_text_preview(fonts, req)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedFile {
    pub file_name: String,
    pub bytes: Vec<u8>,
}

fn unknown_format(id: &str) -> UserMessage {
    UserMessage::new(
        "unknown_format",
        &format!("不認得「{id}」這種檔案格式。"),
        Some("請重新整理頁面，再選一次格式。"),
    )
}

/// 輸出單一格式。函式內會再次檢查是否落在框的安全範圍內，不信任呼叫端。
pub fn encode(
    plan: &StitchPlan,
    hoop: &Hoop,
    format_id: &str,
    design_name: &str,
    sequence: u32,
) -> Result<EncodedFile, UserMessage> {
    let mut files = encode_all(plan, hoop, &[format_id.to_string()], design_name, sequence)?;
    Ok(files.remove(0))
}

/// 一次輸出多種格式。`format_ids` 為空代表所有已註冊的格式。同一次輸出共用主檔名。
pub fn encode_all(
    plan: &StitchPlan,
    hoop: &Hoop,
    format_ids: &[String],
    design_name: &str,
    sequence: u32,
) -> Result<Vec<EncodedFile>, UserMessage> {
    validate_plan(plan, hoop)?;
    let registry = Registry::default();
    let formats: Vec<_> = if format_ids.is_empty() {
        registry.list()
    } else {
        format_ids
            .iter()
            .map(|id| registry.get(id).ok_or_else(|| unknown_format(id)))
            .collect::<Result<_, _>>()?
    };
    let stem = file_stem(design_name, sequence);
    formats
        .into_iter()
        .map(|f| {
            let bytes = f.encode(plan, &stem).map_err(|e| {
                UserMessage::new("encode_failed", &e.to_string(), Some("請重新產生一次。"))
            })?;
            Ok(EncodedFile {
                file_name: file_name(&stem, f.extension()),
                bytes,
            })
        })
        .collect()
}
