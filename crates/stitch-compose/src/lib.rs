//! 多物件合成：一個設計由好幾個物件（文字、圖片、簡單形狀）組成，
//! 每個物件有自己的位置、旋轉與縮放；合起來一起放進繡框、一起檢查。
//!
//! 座標：0.1mm，以框中心為原點，y 軸向下。物件的位置是它「中心」離框中心的偏移。

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use stitch_core::hoop::Hoop;
use stitch_core::{ThreadColor, UserMessage};
use stitch_gen::design::{build_plan_styled, BlendPair, ColoredShape, PreviewResult, Settings};
use stitch_gen::min_size::{enlarge_needed, shrink_limit, MinSizeParams};
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
        /// 繞成弧形：整行字跨過幾度（正的像彩虹往上拱，負的像笑臉往下彎，0 是平的）
        #[serde(default)]
        curve: f64,
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
    /// 手繪：每一筆是一串點（0.1mm，相對於物件中心）。
    Draw {
        strokes: Vec<Vec<[f64; 2]>>,
        /// 線的粗細（0.1mm）
        width: f64,
        color: ThreadColor,
        /// 封起來的圖形要不要把中間填滿
        #[serde(default)]
        filled: bool,
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
    /// 左右翻轉（在旋轉之前）
    #[serde(default)]
    pub flip_x: bool,
    /// 上下翻轉（在旋轉之前）
    #[serde(default)]
    pub flip_y: bool,
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
    hoop: &Hoop,
) -> Result<Built, UserMessage> {
    match &item.content {
        Content::Text {
            text,
            font_id,
            height,
            color,
            max_width,
            curve,
        } => {
            let layout = max_width.map_or(Layout::SingleLine, |w| Layout::Wrap { max_width: w });
            let mut laid = text_to_polygon(fonts, font_id, text, *height, layout)?;
            let mut notes = Vec::new();
            if max_width.is_none() {
                if let Some(wrapped) =
                    wrap_if_cramped(fonts, font_id, text, *height, &laid, item.scale, hoop)
                {
                    laid = wrapped;
                    notes.push(UserMessage::new(
                        "wrapped",
                        "字比較多，一行放不下，已經幫你分成幾行。",
                        Some("想要一行就好的話，可以把字改小一點，或換比較大的框。"),
                    ));
                }
            }
            let list = |cs: &[char]| cs.iter().map(|c| format!("「{c}」")).collect::<String>();
            if !laid.missing.is_empty() {
                notes.push(UserMessage::new(
                    "missing_glyphs",
                    &format!("這個字型沒有{}，已經略過。", list(&laid.missing)),
                    Some("想要繡這些字的話，請換一個字型試試。"),
                ));
            }
            let polygon = if curve.abs() > 0.5 {
                warp_arc(&laid.polygon, *curve)
            } else {
                laid.polygon
            };
            Ok(Built {
                size: polygon_size(&polygon),
                shapes: vec![ColoredShape {
                    polygon,
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
        Content::Draw {
            strokes,
            width,
            color,
            filled,
        } => {
            let polygon = draw_polygon(strokes, *width, *filled);
            if polygon.is_empty() {
                return Err(problem(
                    "empty_drawing",
                    "還沒有畫任何東西。",
                    "請用手指在畫面上畫幾筆。",
                ));
            }
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

/// 讓一個環的繞向固定（面積為正），這樣好幾塊疊在一起時用「非零」規則會合成一塊。
fn orient(mut ring: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    let area: f64 = ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum();
    if area < 0.0 {
        ring.reverse();
    }
    ring
}

/// 手繪的點通常抖、而且很密：先抹掉太近的點，再用兩輪切角讓曲線順。
fn smooth_stroke(points: &[[f64; 2]], min_gap: f64) -> Vec<[f64; 2]> {
    let mut pts: Vec<[f64; 2]> = Vec::new();
    for &p in points {
        if pts
            .last()
            .is_none_or(|q| (q[0] - p[0]).hypot(q[1] - p[1]) >= min_gap)
        {
            pts.push(p);
        }
    }
    if pts.len() < 3 {
        return pts;
    }
    for _ in 0..2 {
        let mut next = vec![pts[0]];
        for w in pts.windows(2) {
            next.push([
                0.75 * w[0][0] + 0.25 * w[1][0],
                0.75 * w[0][1] + 0.25 * w[1][1],
            ]);
            next.push([
                0.25 * w[0][0] + 0.75 * w[1][0],
                0.25 * w[0][1] + 0.75 * w[1][1],
            ]);
        }
        next.push(*pts.last().unwrap());
        pts = next;
    }
    pts
}

/// 一筆線加上粗細，變成一塊形狀：每一小段一個長方形，轉角與兩端用圓形接起來。
fn stroke_pieces(pts: &[[f64; 2]], width: f64) -> Vec<Vec<[f64; 2]>> {
    let r = width / 2.0;
    let disc = |c: [f64; 2]| -> Vec<[f64; 2]> {
        orient(
            (0..16)
                .map(|i| {
                    let a = std::f64::consts::TAU * i as f64 / 16.0;
                    [c[0] + r * a.cos(), c[1] + r * a.sin()]
                })
                .collect(),
        )
    };
    let mut out: Vec<Vec<[f64; 2]>> = Vec::new();
    if let [only] = pts {
        out.push(disc(*only));
        return out;
    }
    for w in pts.windows(2) {
        let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
        let len = dx.hypot(dy);
        if len < 1e-9 {
            continue;
        }
        let (nx, ny) = (-dy / len * r, dx / len * r);
        out.push(orient(vec![
            [w[0][0] + nx, w[0][1] + ny],
            [w[1][0] + nx, w[1][1] + ny],
            [w[1][0] - nx, w[1][1] - ny],
            [w[0][0] - nx, w[0][1] - ny],
        ]));
    }
    // 轉角用圓接起來；為了不產生太多環，只在方向變化大的地方加
    for i in 0..pts.len() {
        let turn = if i == 0 || i == pts.len() - 1 {
            true
        } else {
            let a = (pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]);
            let b = (pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1]);
            let cos = (a.0 * b.0 + a.1 * b.1) / (a.0.hypot(a.1) * b.0.hypot(b.1)).max(1e-9);
            cos < 0.97
        };
        if turn {
            out.push(disc(pts[i]));
        }
    }
    out
}

/// 手繪的所有筆畫合成一塊多邊形。封起來的圖形選了「填滿」時，連中間一起填。
fn draw_polygon(strokes: &[Vec<[f64; 2]>], width: f64, filled: bool) -> Polygon {
    let width = width.max(1.0);
    let mut rings: Vec<Vec<[f64; 2]>> = Vec::new();
    for stroke in strokes {
        let pts = smooth_stroke(stroke, width * 0.25);
        if pts.is_empty() {
            continue;
        }
        rings.extend(stroke_pieces(&pts, width));
        let closed = pts.len() >= 8
            && (pts[0][0] - pts[pts.len() - 1][0]).hypot(pts[0][1] - pts[pts.len() - 1][1])
                <= width * 2.0;
        if filled && closed {
            rings.push(orient(pts.clone()));
        }
    }
    Polygon::new(rings).normalized()
}

/// 左右、上下翻轉（繞物件中心）。
fn flip(p: &Polygon, x: bool, y: bool) -> Polygon {
    if !x && !y {
        return p.clone();
    }
    let (sx, sy) = (if x { -1.0 } else { 1.0 }, if y { -1.0 } else { 1.0 });
    Polygon::new(
        p.rings
            .iter()
            .map(|r| r.iter().map(|q| [q[0] * sx, q[1] * sy]).collect())
            .collect(),
    )
    .normalized()
}

/// 邊太長會在彎的時候變成折線：先把每條邊切成不超過這個長度（0.1mm）。
const WARP_MAX_EDGE: f64 = 15.0;

/// 把一行字繞成弧形：橫向的位置變成角度、高度變成離圓心的距離，字仍然是立著的。
/// `sweep_deg` 是整行字跨過的角度；正的往上拱（像彩虹），負的往下彎（像笑臉）。
fn warp_arc(poly: &Polygon, sweep_deg: f64) -> Polygon {
    let Some((min, max)) = poly.bounds() else {
        return poly.clone();
    };
    let width = (max[0] - min[0]).max(1.0);
    let radius = width / sweep_deg.abs().to_radians();
    let up = sweep_deg > 0.0;
    let map = |q: [f64; 2]| -> [f64; 2] {
        let theta = q[0] / radius;
        // 往下彎時上下顛倒著算，再把結果翻回來
        let h = if up { q[1] } else { -q[1] };
        let r = radius - h;
        let (x, y) = (r * theta.sin(), radius - r * theta.cos());
        [x, if up { y } else { -y }]
    };
    let rings: Vec<Vec<[f64; 2]>> = poly
        .rings
        .iter()
        .map(|ring| {
            let mut out = Vec::new();
            for (i, a) in ring.iter().enumerate() {
                let b = ring[(i + 1) % ring.len()];
                let n = ((a[0] - b[0]).hypot(a[1] - b[1]) / WARP_MAX_EDGE)
                    .ceil()
                    .max(1.0) as usize;
                for k in 0..n {
                    let t = k as f64 / n as f64;
                    out.push(map([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]));
                }
            }
            out
        })
        .collect();
    let warped = Polygon::new(rings);
    // 重新把外框中心放回原點
    match warped.bounds() {
        Some((a, b)) => warped.translate(-(a[0] + b[0]) / 2.0, -(a[1] + b[1]) / 2.0),
        None => warped,
    }
}

/// 單行得縮到比這個倍率還小，就考慮改成分行。
const WRAP_IF_BELOW: f64 = 0.75;
/// 分行後的字要比單行大這麼多倍，才值得分行。
const WRAP_MUST_GAIN: f64 = 1.15;

/// 單行放進框得縮很小時，改成依框寬分行；分行沒有明顯比較大就維持單行。
fn wrap_if_cramped(
    fonts: &FontSet,
    font_id: &str,
    text: &str,
    height: f64,
    single: &stitch_text::TextLayout,
    scale: f64,
    hoop: &Hoop,
) -> Option<stitch_text::TextLayout> {
    let inner = match hoop.shape {
        stitch_core::hoop::HoopShape::Rect => 1.0,
        _ => std::f64::consts::FRAC_1_SQRT_2,
    };
    let safe_w = (hoop.width - 2 * hoop.margin) as f64 * inner;
    let safe_h = (hoop.height - 2 * hoop.margin) as f64 * inner;
    let user = if scale > 0.0 { scale } else { 1.0 };
    let fit = |l: &stitch_text::TextLayout| {
        let [w, h] = polygon_size(&l.polygon);
        (safe_w / (w * user).max(1.0))
            .min(safe_h / (h * user).max(1.0))
            .min(1.0)
    };
    let eff_single = fit(single);
    if eff_single >= WRAP_IF_BELOW {
        return None;
    }
    let wrapped = text_to_polygon(
        fonts,
        font_id,
        text,
        height,
        Layout::Wrap {
            max_width: safe_w / user,
        },
    )
    .ok()?;
    (fit(&wrapped) > eff_single * WRAP_MUST_GAIN).then_some(wrapped)
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
    let out = image_shapes(req)?;
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
    // 每個文字物件：整體縮小之前的實際倍率，與「最少要多大才繡得出來」（字高與筆畫夠寬兩者中較嚴的）
    let mut text_limits: Vec<(f64, f64)> = Vec::new();
    let min_params = MinSizeParams {
        min_text_height: s.min_text_height,
        min_stroke_width: 0.0,
    };
    // 文字要被縮小時，筆畫也不能細到繡不出來
    let full_params = MinSizeParams {
        min_text_height: s.min_text_height,
        min_stroke_width: s.min_stroke_width,
    };

    for item in &req.items {
        let user_scale = if item.scale > 0.0 { item.scale } else { 1.0 };
        let built = match build_item(fonts, item, images, s, &req.hoop) {
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
                let raw = Polygon::new(
                    built
                        .shapes
                        .iter()
                        .flat_map(|c| c.polygon.rings.clone())
                        .collect(),
                );
                let floor = shrink_limit(&raw, effective, *height, &full_params);
                text_limits.push((effective, floor));
            }
        }
        notes.extend(built.notes.iter().cloned());
        for (shape, blend) in built.shapes.into_iter().zip(built.blends) {
            let polygon = flip(&shape.polygon.scale(effective), item.flip_x, item.flip_y)
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
        if text_limits.iter().any(|&(e, floor)| e * k < floor - 1e-6) {
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
