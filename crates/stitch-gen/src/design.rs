//! 把「帶顏色的多邊形」組成完整的針跡計畫：
//! 放大到最小可繡尺寸 → 縮小到放進繡框 → 拉伸補償 → 逐個相連形狀鋪底針、填充、輪廓 → 跳針剪線。
//! 文字與圖片兩條路徑共用。

use crate::fill::{fill, FillParams};
use crate::fit::{fit_polygon, FitResult};
use crate::min_size::{enlarge_needed, shrink_limit, MinSizeParams};
use crate::outline::outline;
use crate::stroke::{center_run, plan_stroke, principal_angle, run_stitch, satin, StrokeKind};
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
        }
    }
}

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
    let shapes: Vec<&ColoredShape> = shapes.iter().filter(|c| !c.polygon.is_empty()).collect();
    if shapes.is_empty() || reference_height <= 0.0 {
        return PreviewResult::blocked(UserMessage::new(
            "empty_design",
            "還沒有可以繡的內容。",
            Some("請先輸入文字或選一張圖。"),
        ));
    }
    let owned: Vec<ColoredShape> = shapes.into_iter().cloned().collect();
    let mut adjustments = Vec::new();

    // 1. 太小就放大（多數情況只檢查一次）
    let min_params = MinSizeParams {
        min_text_height: s.min_text_height,
        min_stroke_width: s.min_stroke_width,
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
    };
    let mut blocks = Vec::new();
    for shape in &owned {
        let scaled = shape.polygon.scale(final_scale);
        // 先把每一塊相連的形狀各自繡成一組針跡，再依位置排順序（減少跳針）
        let mut items: Vec<(Polygon, Vec<Run>)> = Vec::new();
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
                        runs.push(center_run(line, 25.0));
                        runs.extend(satin(line, s.pull_comp.max(0.0)));
                    }
                }
                items.push((base, runs));
                continue;
            }
            let expanded = base.offset(s.pull_comp.max(0.0));
            for part in expanded.shapes() {
                let angle = if s.shape_aware {
                    principal_angle(&part).unwrap_or(s.fill_angle)
                } else {
                    s.fill_angle
                };
                let fp = FillParams {
                    angle_deg: angle,
                    ..fill_params
                };
                let mut runs: Vec<Run> = underlay(&part, angle);
                runs.extend(fill(&part, &fp));
                if s.edge_outline {
                    runs.extend(outline(&part, 25.0));
                }
                items.push((part, runs));
            }
        }
        let runs: Vec<Run> = order_nearest(items).into_iter().flatten().collect();
        blocks.push(ColorBlock {
            color: shape.color.clone(),
            runs,
        });
    }

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
