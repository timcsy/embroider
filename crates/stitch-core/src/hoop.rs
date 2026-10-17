use crate::{Point, UserMessage};
use serde::{Deserialize, Serialize};

/// 繡框上限：300mm。
pub const MAX_HOOP_SIZE: i32 = 3000;
/// 預設安全邊距：5mm。
pub const DEFAULT_MARGIN: i32 = 50;
const MAX_NAME_CHARS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HoopShape {
    Rect,
    Circle,
    Ellipse,
}

/// 繡框。設計座標以框中心為原點。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hoop {
    pub id: String,
    pub name: String,
    /// 可繡寬（0.1mm）
    pub width: i32,
    /// 可繡高（0.1mm）
    pub height: i32,
    pub shape: HoopShape,
    /// 安全邊距（0.1mm）
    pub margin: i32,
    pub builtin: bool,
}

impl Hoop {
    pub fn can_delete(&self) -> bool {
        !self.builtin
    }

    /// 點是否落在「可繡區內縮安全邊距」之內（含邊界）。
    pub fn safe_contains(&self, p: Point) -> bool {
        let hw = (self.width - 2 * self.margin) as f64 / 2.0;
        let hh = (self.height - 2 * self.margin) as f64 / 2.0;
        if hw < 0.0 || hh < 0.0 {
            return false;
        }
        let (x, y) = (p.x as f64, p.y as f64);
        match self.shape {
            HoopShape::Rect => x.abs() <= hw && y.abs() <= hh,
            HoopShape::Circle | HoopShape::Ellipse => {
                if hw == 0.0 || hh == 0.0 {
                    return x == 0.0 && y == 0.0;
                }
                (x / hw).powi(2) + (y / hh).powi(2) <= 1.0 + 1e-9
            }
        }
    }
}

fn err(code: &str, text: &str, hint: &str) -> UserMessage {
    UserMessage::new(code, text, Some(hint))
}

/// 檢查新增或編輯的框是否合理。
pub fn validate_hoop(h: &Hoop) -> Result<(), UserMessage> {
    let name_len = h.name.trim().chars().count();
    if name_len == 0 {
        return Err(err(
            "hoop_name_empty",
            "請幫這個框取個名字。",
            "例如「10×10 方框」。",
        ));
    }
    if h.name.chars().count() > MAX_NAME_CHARS {
        return Err(err(
            "hoop_name_long",
            "名字太長了。",
            "請控制在 20 個字以內。",
        ));
    }
    if h.width <= 0 || h.height <= 0 {
        return Err(err(
            "hoop_size_zero",
            "框的寬度和高度要大於零。",
            "請量一下內框可以繡的範圍再填。",
        ));
    }
    if h.width > MAX_HOOP_SIZE || h.height > MAX_HOOP_SIZE {
        return Err(err(
            "hoop_size_big",
            "這個尺寸比一般的繡框大很多。",
            "請確認單位是公釐，最大可填 300 公釐。",
        ));
    }
    if h.shape == HoopShape::Circle && h.width != h.height {
        return Err(err(
            "hoop_circle_unequal",
            "圓形的框，寬度和高度要一樣。",
            "請把兩個數字填成一樣，或改選橢圓。",
        ));
    }
    if h.margin < 0 {
        return Err(err(
            "hoop_margin_negative",
            "安全邊距不能是負的。",
            "請填零或更大的數字。",
        ));
    }
    if h.margin * 2 >= h.width.min(h.height) {
        return Err(err(
            "hoop_margin_big",
            "安全邊距太大，框裡面沒有地方可以繡了。",
            "請把安全邊距調小一點。",
        ));
    }
    Ok(())
}

fn builtin(id: &str, name: &str, width: i32, height: i32) -> Hoop {
    Hoop {
        id: id.to_string(),
        name: name.to_string(),
        width,
        height,
        shape: HoopShape::Rect,
        margin: DEFAULT_MARGIN,
        builtin: true,
    }
}

/// bernette B70 隨機附的三種原廠框（尺寸取自零售商公開資訊，實際以機器為準）。
pub fn builtin_hoops() -> Vec<Hoop> {
    vec![
        builtin("b70-large", "大框（原廠）", 1600, 2600),
        builtin("b70-medium", "中框（原廠）", 1200, 1800),
        builtin("b70-small", "小框（原廠）", 500, 750),
    ]
}
