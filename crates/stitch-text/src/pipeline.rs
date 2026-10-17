use crate::{text_to_polygon, FontSet, Layout};
use serde::{Deserialize, Serialize};
use stitch_core::hoop::Hoop;
use stitch_core::{ThreadColor, UserMessage};
use stitch_gen::design::{build_plan, ColoredShape, PreviewResult, Settings};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LayoutSpec {
    SingleLine,
    Wrap { max_width: f64 },
}

impl From<LayoutSpec> for Layout {
    fn from(l: LayoutSpec) -> Self {
        match l {
            LayoutSpec::SingleLine => Layout::SingleLine,
            LayoutSpec::Wrap { max_width } => Layout::Wrap { max_width },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextRequest {
    pub text: String,
    pub font_id: String,
    /// 字級（0.1mm）
    pub height: f64,
    pub color: ThreadColor,
    pub layout: LayoutSpec,
    pub hoop: Hoop,
    #[serde(default)]
    pub settings: Settings,
}

fn list(chars: &[char]) -> String {
    chars
        .iter()
        .map(|c| format!("「{c}」"))
        .collect::<Vec<_>>()
        .join("")
}

fn build_once(
    fonts: &FontSet,
    req: &TextRequest,
    layout: Layout,
) -> Result<(PreviewResult, Vec<UserMessage>), UserMessage> {
    let laid_out = text_to_polygon(fonts, &req.font_id, &req.text, req.height, layout)?;
    let mut notes: Vec<UserMessage> = Vec::new();
    if !laid_out.missing.is_empty() {
        notes.push(UserMessage::new(
            "missing_glyphs",
            &format!("這個字型沒有{}，已經略過。", list(&laid_out.missing)),
            Some("想要繡這些字的話，請換一個字型試試。"),
        ));
    }
    if !laid_out.fallback_used.is_empty() {
        notes.push(UserMessage::new(
            "fallback_font",
            &format!("{}改用別的字型補上了。", list(&laid_out.fallback_used)),
            None,
        ));
    }
    let shapes = [ColoredShape {
        polygon: laid_out.polygon,
        color: req.color.clone(),
    }];
    Ok((
        build_plan(&shapes, req.height, &req.hoop, &req.settings),
        notes,
    ))
}

type Built = Result<(PreviewResult, Vec<UserMessage>), UserMessage>;

/// 以指定排版建出計畫。為了筆畫夠寬而放大後字會變寬，自動換行的寬度限制要在放大後仍然成立，
/// 所以放大了就用較窄的寬度重排一次。
fn run(fonts: &FontSet, req: &TextRequest, layout: Layout) -> Built {
    let (mut result, mut notes) = build_once(fonts, req, layout)?;
    if let (Layout::Wrap { max_width }, true) = (
        layout,
        result.blocked.is_none() && result.enlarge_scale > 1.0 + 1e-6,
    ) {
        (result, notes) = build_once(
            fonts,
            req,
            Layout::Wrap {
                max_width: max_width / result.enlarge_scale,
            },
        )?;
    }
    if result.blocked.is_none() {
        notes.append(&mut result.adjustments);
        result.adjustments = notes;
        notes = Vec::new();
    }
    Ok((result, notes))
}

/// 相對於使用者設定的大小，最後實際用的倍率。
fn effective_scale(r: &PreviewResult) -> f64 {
    r.enlarge_scale * r.fitted_scale
}

/// 單行要縮到比這個倍率還小，就考慮改成分行。
const WRAP_IF_BELOW: f64 = 0.75;
/// 分行後的字要比單行大這麼多倍，才值得分行。
const WRAP_MUST_GAIN: f64 = 1.15;

/// 文字 → 針跡計畫。任何無法輸出的情況都以 `blocked` 回報白話原因。
///
/// 預設單行；單行放不下、或得縮得很小時，自動改成依框寬分行（名字這種短文字不受影響）。
pub fn build_text_preview(fonts: &FontSet, req: &TextRequest) -> PreviewResult {
    let layout: Layout = req.layout.into();
    let (single, _) = match run(fonts, req, layout) {
        Ok(v) => v,
        Err(m) => return PreviewResult::blocked(m),
    };

    let cramped = single.blocked.is_some()
        && single.blocked.as_ref().map(|m| m.code.as_str()) == Some("too_big_for_hoop")
        || (single.blocked.is_none() && effective_scale(&single) < WRAP_IF_BELOW);
    if layout != Layout::SingleLine || !cramped {
        return single;
    }

    let safe_width = (req.hoop.width - 2 * req.hoop.margin) as f64;
    let wrapped_layout = Layout::Wrap {
        max_width: safe_width,
    };
    if let Ok((mut wrapped, _)) = run(fonts, req, wrapped_layout) {
        let better = wrapped.blocked.is_none()
            && (single.blocked.is_some()
                || effective_scale(&wrapped) > effective_scale(&single) * WRAP_MUST_GAIN);
        if better {
            wrapped.adjustments.insert(
                0,
                UserMessage::new(
                    "wrapped",
                    "字比較多，一行放不下，已經幫你分成幾行。",
                    Some("想要一行就好的話，可以把字改小一點，或換比較大的框。"),
                ),
            );
            return wrapped;
        }
    }
    single
}
