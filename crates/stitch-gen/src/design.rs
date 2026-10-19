//! 把「帶顏色的多邊形」組成完整的針跡計畫：
//! 放大到最小可繡尺寸 → 縮小到放進繡框 → 拉伸補償 → 逐個相連形狀鋪底針、填充、輪廓 → 跳針剪線。
//! 文字與圖片兩條路徑共用。

use crate::fill::{best_fill_angle, contour_fill, fill, FillParams};
use crate::fit::{fit_polygon, FitResult};
use crate::min_size::{enlarge_needed, shrink_limit, MinSizeParams};
use crate::outline::outline;
use crate::stroke::{
    flow_fill, mean_width, plan_stroke, principal_angle, run_stitch, satin, satin_underlay,
    StrokeKind,
};
use crate::travel::{assemble, group_by_color, ColorBlock, TravelParams};
use crate::underlay::underlay;
use crate::Run;
use serde::{Deserialize, Serialize};
use stitch_core::hoop::Hoop;
use stitch_core::{validate_plan, StitchPlan, ThreadColor, UserMessage};
use stitch_geom::Polygon;

/// 可調整的參數，單位 0.1mm。預設值見各欄位。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// 最小字高（4mm）
    pub min_text_height: f64,
    /// 最小筆畫寬（0.8mm）
    pub min_stroke_width: f64,
    /// 拉伸補償：填充區域向外多繡的量（0.2mm）
    pub pull_comp: f64,
    /// 填充方向（度）
    pub fill_angle: f64,
    /// 行距（0.4mm）
    pub row_spacing: f64,
    /// 單針最大長度（3mm）
    pub max_stitch: f64,
    /// 超過此距離改用跳針（3mm）
    pub jump_threshold: f64,
    /// 超過此距離跳針前先剪線（8mm）
    pub trim_threshold: f64,
    /// 填充完是否沿邊緣再走一圈
    pub edge_outline: bool,
    /// 依形狀選繡法：細線走中線平針、窄柱走緞面、寬的填充，且填充順著形狀的長邊。
    /// 文字用字型的筆畫，不需要；圖片才開。
    pub shape_aware: bool,
    /// 圓潤、沒有明顯長邊的填充區域改用輪廓填充（一圈一圈順著邊緣）。需要 `shape_aware`。
    pub contour_fill: bool,
    /// 彎曲的寬帶狀區域，針跡順著彎度走（需 `shape_aware`）
    pub flow_fill: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            min_text_height: 40.0,
            min_stroke_width: 8.0,
            pull_comp: 2.0,
            fill_angle: 45.0,
            row_spacing: 4.0,
            max_stitch: 30.0,
            jump_threshold: 30.0,
            trim_threshold: 80.0,
            edge_outline: true,
            shape_aware: false,
            contour_fill: false,
            flow_fill: false,
        }
    }
}

/// 混色的區域至少要這麼寬（3mm）；更窄的兩列就塞滿，交錯只會變成條紋。
const BLEND_MIN_WIDTH: f64 = 30.0;

/// 一塊形狀的針跡：（形狀本身的線色, 混色的第一條線, 混色的第二條線）。
type RunSets = (Vec<Run>, Vec<Run>, Vec<Run>);

/// 混色用的兩條線。
pub type BlendPair = [ThreadColor; 2];

/// 以框中心為原點、y 軸向下的多邊形，加上要用的線色。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColoredShape {
    pub polygon: Polygon,
    pub color: ThreadColor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreviewResult {
    pub plan: StitchPlan,
    /// 為了放進框而額外套用的縮放（1.0 表示沒有縮小）
    pub fitted_scale: f64,
    /// 為了達到最小可繡尺寸而套用的放大倍率（1.0 表示沒有放大）
    pub enlarge_scale: f64,
    /// 自動調整的白話說明
    pub adjustments: Vec<UserMessage>,
    /// 非空代表不可輸出
    pub blocked: Option<UserMessage>,
}

impl PreviewResult {
    pub fn blocked(m: UserMessage) -> Self {
        Self {
            plan: StitchPlan::default(),
            fitted_scale: 1.0,
            enlarge_scale: 1.0,
            adjustments: Vec::new(),
            blocked: Some(m),
        }
    }
}

fn all_rings(shapes: &[ColoredShape], scale: f64) -> Polygon {
    Polygon::new(
        shapes
            .iter()
            .flat_map(|s| s.polygon.scale(scale).rings)
            .collect(),
    )
}

fn center(poly: &Polygon) -> [f64; 2] {
    let (min, max) = poly.bounds().unwrap();
    [(min[0] + max[0]) / 2.0, (min[1] + max[1]) / 2.0]
}

/// 由左上角出發，每次選最近的形狀，減少跳針距離。
fn order_nearest<T>(mut shapes: Vec<(Polygon, T)>) -> Vec<T> {
    let mut out = Vec::with_capacity(shapes.len());
    let mut cur = [f64::MIN / 4.0, f64::MIN / 4.0];
    let mut first = true;
    while !shapes.is_empty() {
        let key = |s: &(Polygon, T)| {
            let c = center(&s.0);
            if first {
                c[0] + c[1]
            } else {
                (c[0] - cur[0]).powi(2) + (c[1] - cur[1]).powi(2)
            }
        };
        let (i, _) = shapes
            .iter()
            .enumerate()
            .map(|(i, s)| (i, key(s)))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        let s = shapes.remove(i);
        cur = center(&s.0);
        first = false;
        out.push(s.1);
    }
    out
}

/// 0.1mm 單位換成公分（1 公分 = 100 單位）。
fn cm(v: f64) -> String {
    format!("{:.1}", v / 100.0)
}

/// 組出針跡計畫。`reference_height` 是使用者設定的字級／圖高（0.1mm），用來判斷是否低於最小字高。
pub fn build_plan(
    shapes: &[ColoredShape],
    reference_height: f64,
    hoop: &Hoop,
    s: &Settings,
) -> PreviewResult {
    build_plan_with_blends(shapes, &vec![None; shapes.len()], reference_height, hoop, s)
}

/// 同 `build_plan`，但每個形狀可以多指定一對「混色線」：夠寬的填充區域改成兩色交錯的列，
/// 看起來是兩色的中間色（例如紅加藍近似紫色），不必為了少數顏色準備更多線色。
/// 做法來自 Brother 的專利 US8473090B2（以兩種線的混合近似色票裡沒有的顏色）。
/// `blends` 與 `shapes` 一一對應；只有夠寬的填充區域會混色，細線、緞面與太窄的區域仍用形狀本身的線色。
pub fn build_plan_with_blends(
    shapes: &[ColoredShape],
    blends: &[Option<BlendPair>],
    reference_height: f64,
    hoop: &Hoop,
    s: &Settings,
) -> PreviewResult {
    let pairs: Vec<(&ColoredShape, Option<&BlendPair>)> = shapes
        .iter()
        .zip(
            blends
                .iter()
                .map(Option::as_ref)
                .chain(std::iter::repeat(None)),
        )
        .filter(|(c, _)| !c.polygon.is_empty())
        .collect();
    if pairs.is_empty() || reference_height <= 0.0 {
        return PreviewResult::blocked(UserMessage::new(
            "empty_design",
            "還沒有可以繡的內容。",
            Some("請先輸入文字或選一張圖。"),
        ));
    }
    let owned: Vec<ColoredShape> = pairs.iter().map(|p| p.0.clone()).collect();
    let blend_of: Vec<Option<BlendPair>> = pairs.iter().map(|p| p.1.cloned()).collect();
    let mut adjustments = Vec::new();

    // 1. 太小就放大（多數情況只檢查一次）
    // 依形狀選繡法時，比最小筆畫細的線會改走單線平針，不需要為了它們放大整張圖，
    // 也不該因為縮小後線更細就擋下來。
    let min_params = MinSizeParams {
        min_text_height: s.min_text_height,
        min_stroke_width: if s.shape_aware {
            0.0
        } else {
            s.min_stroke_width
        },
    };
    let unit = all_rings(&owned, 1.0);
    let desired = enlarge_needed(&unit, reference_height, &min_params);
    if desired > 1.0 + 1e-6 {
        adjustments.push(UserMessage::new(
            "enlarged",
            &format!(
                "太小了，繡出來會糊，已幫你放大到約 {} 公分高。",
                cm(reference_height * desired)
            ),
            None,
        ));
    }

    // 2. 太大就縮小到放進框（拉伸補償會多出一點，所以最多重算幾次）。
    //    縮小後仍要繡得出來；這個下限只在真的需要縮小時才計算。
    let mut required: Option<f64> = None;
    let mut f = 1.0;
    for _ in 0..4 {
        let scale = desired * f;
        let candidate = all_rings(&owned, scale).offset(s.pull_comp.max(0.0));
        let min_rel = required.map_or(0.0, |r| r / scale);
        match fit_polygon(&candidate, hoop, min_rel) {
            FitResult::Fits => break,
            FitResult::Scaled(k) => {
                if required.is_none() {
                    let r = shrink_limit(&unit, desired, reference_height, &min_params);
                    required = Some(r);
                    // 有了下限，重新判斷這次縮小是否仍然可以接受
                    if let FitResult::Blocked(m) = fit_polygon(&candidate, hoop, r / scale) {
                        return PreviewResult::blocked(m);
                    }
                }
                f *= k;
            }
            FitResult::Blocked(m) => return PreviewResult::blocked(m),
        }
    }
    let final_scale = desired * f;
    if f < 1.0 - 1e-9 {
        adjustments.push(UserMessage::new(
            "shrunk",
            &format!(
                "放不進「{}」，已幫你縮小到原來的 {}%。",
                hoop.name,
                (f * 100.0).floor() as i32
            ),
            Some("想要大一點的話，可以換比較大的框。"),
        ));
    }

    // 3. 逐個相連的形狀鋪針
    let fill_params = FillParams {
        row_spacing: s.row_spacing,
        max_stitch: s.max_stitch,
        min_stitch: 5.0,
        angle_deg: s.fill_angle,
        y_offset: 0.0,
    };
    // 填充先繡、細線與輪廓（平針、緞面）最後繡：線壓在最上面，邊緣才清楚（Brother US8473090B2「銳利的部分最後繡」）
    let mut blocks = Vec::new();
    let mut line_blocks = Vec::new();
    for (shape, blend) in owned.iter().zip(&blend_of) {
        let scaled = shape.polygon.scale(final_scale);
        // 先把每一塊相連的形狀各自繡成一組針跡，再依位置排順序（減少跳針）
        let mut items: Vec<(Polygon, RunSets)> = Vec::new();
        let mut line_items: Vec<(Polygon, RunSets)> = Vec::new();
        let bases = if s.shape_aware {
            scaled.shapes()
        } else {
            vec![scaled.clone()]
        };
        for base in bases {
            let stroke = if s.shape_aware {
                plan_stroke(&base)
            } else {
                None
            };
            if let Some((kind, lines)) = stroke {
                let mut runs: Vec<Run> = Vec::new();
                for line in &lines {
                    if kind == StrokeKind::Run {
                        runs.extend(run_stitch(line, 25.0));
                    } else {
                        runs.extend(satin_underlay(line));
                        runs.extend(satin(line, s.pull_comp.max(0.0)));
                    }
                }
                line_items.push((base, (runs, Vec::new(), Vec::new())));
                continue;
            }
            let expanded = base.offset(s.pull_comp.max(0.0));
            for part in expanded.shapes() {
                // 依形狀選角度：碎片最少的；一樣時偏好順著長邊
                let axis = if s.shape_aware {
                    principal_angle(&part)
                } else {
                    None
                };
                let round = s.shape_aware && axis.is_none();
                let angle = if s.shape_aware {
                    let prefer = axis.unwrap_or(s.fill_angle);
                    best_fill_angle(&part, s.row_spacing, prefer)
                } else {
                    s.fill_angle
                };
                // 混色：兩色各繡一半的列（行距加倍），第二色錯開一個行距，合起來等於一般的密度
                // 太窄的區域（兩列就塞滿）混色只會變成條紋，不混
                let part_blend = blend
                    .as_ref()
                    .filter(|_| mean_width(&part) >= BLEND_MIN_WIDTH);
                let step = if part_blend.is_some() { 2.0 } else { 1.0 };
                let fp = FillParams {
                    angle_deg: angle,
                    row_spacing: s.row_spacing * step,
                    ..fill_params
                };
                let mut runs: Vec<Run> = underlay(&part, angle);
                // 圓潤、沒有明顯長邊的形狀（沒有主軸）：輪廓填充，針跡順著邊緣；已含最外圈，不必再走邊
                if s.shape_aware && s.contour_fill && part_blend.is_none() && round {
                    runs.extend(contour_fill(&part, &fp));
                } else {
                    let flowed = (s.shape_aware && s.flow_fill && part_blend.is_none())
                        .then(|| flow_fill(&part, &fp))
                        .flatten();
                    runs.extend(flowed.unwrap_or_else(|| fill(&part, &fp)));
                    if s.edge_outline {
                        runs.extend(outline(&part, 25.0));
                    }
                }
                if part_blend.is_some() {
                    let second = fill(
                        &part,
                        &FillParams {
                            y_offset: s.row_spacing,
                            ..fp
                        },
                    );
                    items.push((part, (Vec::new(), runs, second)));
                } else {
                    items.push((part, (runs, Vec::new(), Vec::new())));
                }
            }
        }
        let mut plain: Vec<Run> = Vec::new();
        let mut first: Vec<Run> = Vec::new();
        let mut second: Vec<Run> = Vec::new();
        for (p, a, b) in order_nearest(items) {
            plain.extend(p);
            first.extend(a);
            second.extend(b);
        }
        // 細線另成一塊，排在所有填充之後
        let line_runs: Vec<Run> = order_nearest(line_items)
            .into_iter()
            .flat_map(|(p, _, _)| p)
            .collect();
        if !line_runs.is_empty() {
            line_blocks.push(ColorBlock {
                color: shape.color.clone(),
                runs: line_runs,
            });
        }
        if !plain.is_empty() {
            blocks.push(ColorBlock {
                color: shape.color.clone(),
                runs: plain,
            });
        }
        if let Some([a, b]) = blend {
            for (color, runs) in [(a, first), (b, second)] {
                if !runs.is_empty() {
                    blocks.push(ColorBlock {
                        color: color.clone(),
                        runs,
                    });
                }
            }
        }
    }

    blocks.extend(line_blocks);
    let travel = TravelParams {
        jump_threshold: s.jump_threshold,
        trim_threshold: s.trim_threshold,
        lock_len: 4.0,
    };
    let plan = assemble(&group_by_color(blocks), &travel);
    if let Err(m) = validate_plan(&plan, hoop) {
        return PreviewResult::blocked(m);
    }
    PreviewResult {
        plan,
        fitted_scale: f,
        enlarge_scale: desired,
        adjustments,
        blocked: None,
    }
}
