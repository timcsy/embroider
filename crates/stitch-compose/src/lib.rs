//! 多物件合成：一個設計由好幾個物件（文字、圖片、簡單形狀）組成，
//! 每個物件有自己的位置、旋轉與縮放；合起來一起放進繡框、一起檢查。
//!
//! 座標：0.1mm，以框中心為原點，y 軸向下。物件的位置是它「中心」離框中心的偏移。

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use stitch_core::hoop::Hoop;
use stitch_core::{ThreadColor, UserMessage};
use stitch_gen::design::{build_plan_styled, BlendPair, ColoredShape, PreviewResult, Settings};
use stitch_gen::min_size::{enlarge_needed, MinSizeParams};
use stitch_geom::Polygon;
use stitch_image::{image_shapes, ImageRequest, ImageShapes};
use stitch_text::{text_to_polygon, FontSet, Layout};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeKind {
    Circle,
    Rect,
    Heart,
    Star,
    /// 一條直線（粗細用 `line_width`）
    Line,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Content {
    Text {
        text: String,
        font_id: String,
        /// 字級（0.1mm）
        height: f64,
        color: ThreadColor,
        /// 超過這個寬度（0.1mm）就自動換行；沒有就單行
        #[serde(default)]
        max_width: Option<f64>,
    },
    Image {
        /// 對應 `build_composite` 的 `images` 第幾張
        image: usize,
        width: usize,
        height: usize,
        color_count: usize,
        simplify: u32,
        /// 圖長邊的大小（0.1mm）
        size: f64,
    },
    Shape {
        shape: ShapeKind,
        width: f64,
        height: f64,
        color: ThreadColor,
        #[serde(default = "yes")]
        filled: bool,
        /// 空心形狀或直線的粗細（0.1mm）
        #[serde(default = "default_line_width")]
        line_width: f64,
    },
}

fn yes() -> bool {
    true
}
fn default_line_width() -> f64 {
    25.0
}
fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    #[serde(flatten)]
    pub content: Content,
    /// 物件中心離框中心的偏移（0.1mm）
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    /// 旋轉（度，順時針）
    #[serde(default)]
    pub rotation: f64,
    /// 使用者縮放倍率
    #[serde(default = "one")]
    pub scale: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeRequest {
    pub items: Vec<Item>,
    pub hoop: Hoop,
    #[serde(default)]
    pub settings: Settings,
}

/// 每個物件的結果，給畫面畫選取框用。大小都是縮放後、還沒套用「整體縮小放進框」的值。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemInfo {
    pub id: String,
    pub width: f64,
    pub height: f64,
    /// 實際用的縮放（使用者縮放 × 為了繡得出來而放大的倍率）
    pub scale: f64,
    /// 這個物件沒辦法繡的原因（其他物件不受影響）
    pub problem: Option<UserMessage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeResult {
    pub preview: PreviewResult,
    pub items: Vec<ItemInfo>,
}

struct Built {
    shapes: Vec<ColoredShape>,
    blends: Vec<Option<BlendPair>>,
    smart: bool,
    /// 沒縮放前的外框大小
    size: [f64; 2],
    /// 判斷「太小」用的參考高度
    reference: Option<f64>,
    min_text: bool,
    notes: Vec<UserMessage>,
}

fn problem(code: &str, text: &str, hint: &str) -> UserMessage {
    UserMessage::new(code, text, Some(hint))
}

fn polygon_size(p: &Polygon) -> [f64; 2] {
    p.bounds()
        .map_or([0.0, 0.0], |(a, b)| [b[0] - a[0], b[1] - a[1]])
}

fn ellipse(w: f64, h: f64) -> Polygon {
    let n = 72;
    Polygon::new(vec![(0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            [w / 2.0 * a.cos(), h / 2.0 * a.sin()]
        })
        .collect()])
}

fn rect(w: f64, h: f64) -> Polygon {
    Polygon::new(vec![vec![
        [-w / 2.0, -h / 2.0],
        [w / 2.0, -h / 2.0],
        [w / 2.0, h / 2.0],
        [-w / 2.0, h / 2.0],
    ]])
}

/// 把一串點縮放、平移，使外框剛好是 w × h 且置中。
fn fit_ring(pts: Vec<[f64; 2]>, w: f64, h: f64) -> Polygon {
    let (x0, x1) = pts
        .iter()
        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p[0]), a.1.max(p[0])));
    let (y0, y1) = pts
        .iter()
        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p[1]), a.1.max(p[1])));
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let (sx, sy) = (w / (x1 - x0), h / (y1 - y0));
    Polygon::new(vec![pts
        .into_iter()
        .map(|p| [(p[0] - cx) * sx, (p[1] - cy) * sy])
        .collect()])
}

fn heart(w: f64, h: f64) -> Polygon {
    let n = 96;
    let pts = (0..n)
        .map(|i| {
            let t = std::f64::consts::TAU * i as f64 / n as f64;
            [
                16.0 * t.sin().powi(3),
                -(13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos()),
            ]
        })
        .collect();
    fit_ring(pts, w, h)
}

fn star(w: f64, h: f64) -> Polygon {
    let pts = (0..10)
        .map(|i| {
            let a = -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * i as f64 / 5.0;
            let r = if i % 2 == 0 { 1.0 } else { 0.4 };
            [r * a.cos(), r * a.sin()]
        })
        .collect();
    fit_ring(pts, w, h)
}

/// 空心：外框往外、內框往內各半個線寬，內框反向當作洞。
fn hollow(outline: &Polygon, line_width: f64) -> Polygon {
    let half = line_width / 2.0;
    let outer = outline.offset(half);
    let inner = outline.offset(-half);
    let mut rings = outer.rings;
    rings.extend(inner.rings.into_iter().map(|mut r| {
        r.reverse();
        r
    }));
    Polygon::new(rings)
}

fn shape_polygon(kind: ShapeKind, w: f64, h: f64, filled: bool, line_width: f64) -> Polygon {
    let base = match kind {
        ShapeKind::Circle => ellipse(w, h),
        ShapeKind::Rect => rect(w, h),
        ShapeKind::Heart => heart(w, h),
        ShapeKind::Star => star(w, h),
        ShapeKind::Line => return rect(w, line_width),
    };
    if filled {
        base
    } else {
        hollow(&base, line_width)
    }
}

fn build_item(
    fonts: &FontSet,
    item: &Item,
    images: &[Vec<u8>],
    settings: &Settings,
) -> Result<Built, UserMessage> {
    match &item.content {
        Content::Text {
            text,
            font_id,
            height,
            color,
            max_width,
        } => {
            let layout = max_width.map_or(Layout::SingleLine, |w| Layout::Wrap { max_width: w });
            let laid = text_to_polygon(fonts, font_id, text, *height, layout)?;
            let mut notes = Vec::new();
            let list = |cs: &[char]| cs.iter().map(|c| format!("「{c}」")).collect::<String>();
            if !laid.missing.is_empty() {
                notes.push(UserMessage::new(
                    "missing_glyphs",
                    &format!("這個字型沒有{}，已經略過。", list(&laid.missing)),
                    Some("想要繡這些字的話，請換一個字型試試。"),
                ));
            }
            Ok(Built {
                size: polygon_size(&laid.polygon),
                shapes: vec![ColoredShape {
                    polygon: laid.polygon,
                    color: color.clone(),
                }],
                blends: vec![None],
                smart: false,
                reference: Some(*height),
                min_text: true,
                notes,
            })
        }
        Content::Image {
            image,
            width,
            height,
            color_count,
            simplify,
            size,
        } => {
            let rgba = images.get(*image).ok_or_else(|| {
                problem("bad_image", "這張圖打不開。", "請換一張圖，或重新選一次。")
            })?;
            let req = ImageRequest {
                rgba: rgba.clone(),
                width: *width,
                height: *height,
                color_count: *color_count,
                simplify: *simplify,
                size: *size,
                hoop: Hoop {
                    // 圖片只負責轉成形狀，放不放得進框由整體決定
                    ..hoop_placeholder()
                },
                settings: settings.clone(),
            };
            let out = cached_image_shapes(&req)?;
            let long = (*width).max(*height) as f64;
            Ok(Built {
                size: [*size * *width as f64 / long, *size * *height as f64 / long],
                shapes: out.shapes,
                blends: out.blends,
                smart: true,
                reference: Some(*size),
                min_text: false,
                notes: out.notes,
            })
        }
        Content::Shape {
            shape,
            width,
            height,
            color,
            filled,
            line_width,
        } => {
            if *width <= 0.0 || *height <= 0.0 {
                return Err(problem(
                    "bad_size",
                    "形狀的大小要大於零。",
                    "請把它拉大一點。",
                ));
            }
            let polygon = shape_polygon(*shape, *width, *height, *filled, *line_width);
            Ok(Built {
                size: polygon_size(&polygon),
                shapes: vec![ColoredShape {
                    polygon,
                    color: color.clone(),
                }],
                blends: vec![None],
                smart: true,
                reference: None,
                min_text: false,
                notes: Vec::new(),
            })
        }
    }
}

/// 圖片轉形狀很花時間；拖移、旋轉、縮放物件時圖片本身沒變，用快取避免每次重算。
fn cached_image_shapes(req: &ImageRequest) -> Result<ImageShapes, UserMessage> {
    thread_local! {
        static CACHE: RefCell<Vec<(u64, ImageShapes)>> = const { RefCell::new(Vec::new()) };
    }
    const KEEP: usize = 4;
    // FNV-1a：像素加上所有會影響結果的設定
    let mut h: u64 = 0xcbf29ce484222325;
    let mut mix = |bytes: &[u8]| {
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    };
    mix(&req.rgba);
    mix(format!(
        "{}x{} {} {} {} {:?}",
        req.width, req.height, req.color_count, req.simplify, req.size, req.settings
    )
    .as_bytes());
    if let Some(hit) = CACHE.with(|c| c.borrow().iter().find(|e| e.0 == h).map(|e| e.1.clone())) {
        return Ok(hit);
    }
    let out = image_shapes(req).map_err(|b| {
        b.blocked
            .unwrap_or_else(|| problem("bad_image", "這張圖打不開。", "請換一張圖。"))
    })?;
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        c.push((h, out.clone()));
        if c.len() > KEEP {
            c.remove(0);
        }
    });
    Ok(out)
}

fn hoop_placeholder() -> Hoop {
    Hoop {
        id: "compose".into(),
        name: "合成".into(),
        width: 100_000,
        height: 100_000,
        shape: stitch_core::hoop::HoopShape::Rect,
        margin: 0,
        builtin: false,
    }
}

/// 組出針跡計畫。圖片像素放在 `images`（RGBA），物件以 `Content::Image::image` 指定第幾張。
pub fn build_composite(
    fonts: &FontSet,
    req: &CompositeRequest,
    images: &[Vec<u8>],
) -> CompositeResult {
    let s = &req.settings;
    let mut infos: Vec<ItemInfo> = Vec::new();
    let mut shapes: Vec<ColoredShape> = Vec::new();
    let mut blends: Vec<Option<BlendPair>> = Vec::new();
    let mut smart: Vec<bool> = Vec::new();
    let mut notes: Vec<UserMessage> = Vec::new();
    // 每個文字物件的實際字高（0.1mm，整體縮小之前）
    let mut text_heights: Vec<f64> = Vec::new();
    let min_params = MinSizeParams {
        min_text_height: s.min_text_height,
        min_stroke_width: 0.0,
    };

    for item in &req.items {
        let user_scale = if item.scale > 0.0 { item.scale } else { 1.0 };
        let built = match build_item(fonts, item, images, s) {
            Ok(b) => b,
            Err(m) => {
                infos.push(ItemInfo {
                    id: item.id.clone(),
                    width: 0.0,
                    height: 0.0,
                    scale: user_scale,
                    problem: Some(m),
                });
                continue;
            }
        };
        // 太小就放大（只有文字與圖片；形狀大小由使用者決定）
        let mut effective = user_scale;
        if let Some(reference) = built.reference {
            let unit = Polygon::new(
                built
                    .shapes
                    .iter()
                    .flat_map(|c| c.polygon.scale(user_scale).rings)
                    .collect(),
            );
            let e = enlarge_needed(&unit, reference * user_scale, &min_params);
            if e > 1.0 + 1e-6 {
                effective *= e;
                notes.push(UserMessage::new(
                    "enlarged",
                    "有些地方太小，繡出來會糊，已幫你放大。",
                    None,
                ));
            }
        }
        if built.min_text {
            if let Content::Text { height, .. } = &item.content {
                text_heights.push(height * effective);
            }
        }
        notes.extend(built.notes.iter().cloned());
        for (shape, blend) in built.shapes.into_iter().zip(built.blends) {
            let polygon = shape
                .polygon
                .scale(effective)
                .rotate(item.rotation)
                .translate(item.x, item.y);
            shapes.push(ColoredShape {
                polygon,
                color: shape.color,
            });
            blends.push(blend);
            smart.push(built.smart);
        }
        infos.push(ItemInfo {
            id: item.id.clone(),
            width: built.size[0] * effective,
            height: built.size[1] * effective,
            scale: effective,
            problem: None,
        });
    }

    // 整體只負責放進框；各物件的最小尺寸已在上面個別處理
    let settings = Settings {
        min_text_height: 0.0,
        min_stroke_width: 0.0,
        contour_fill: true,
        flow_fill: true,
        field_fill: true,
        ..s.clone()
    };
    let mut preview = build_plan_styled(&shapes, &blends, &smart, 1.0, &req.hoop, &settings);
    if preview.blocked.is_none() {
        // 整體縮小之後，文字不能小到繡不出來
        let k = preview.fitted_scale;
        if text_heights
            .iter()
            .any(|h| h * k < s.min_text_height - 1e-6)
        {
            return CompositeResult {
                preview: PreviewResult::blocked(UserMessage::new(
                    "too_big_for_hoop",
                    &format!(
                        "這個設計放不進「{}」。縮小的話字會太小，繡出來會糊。",
                        req.hoop.name
                    ),
                    Some("請換一個比較大的框，或把東西排得近一點。"),
                )),
                items: infos,
            };
        }
        notes.dedup_by(|a, b| a.code == b.code && a.code == "enlarged");
        notes.append(&mut preview.adjustments);
        preview.adjustments = notes;
    }
    CompositeResult {
        preview,
        items: infos,
    }
}
