use crate::FontSet;
use kurbo::BezPath;
use stitch_core::UserMessage;
use stitch_geom::{flatten_path, Polygon};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Layout {
    /// 單行（文字中的換行字元仍會換行）
    SingleLine,
    /// 超過 `max_width`（0.1mm）自動換行
    Wrap { max_width: f64 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout {
    pub polygon: Polygon,
    /// 所有字型都沒有的字
    pub missing: Vec<char>,
    /// 主要字型沒有、改用其他字型補上的字
    pub fallback_used: Vec<char>,
}

const LINE_HEIGHT: f64 = 1.3;
const FLATTEN_TOLERANCE: f64 = 0.2;

struct Glyph {
    ch: char,
    font: usize,
    id: ttf_parser::GlyphId,
    advance: f64,
}

struct Builder {
    path: BezPath,
    scale: f64,
    dx: f64,
    dy: f64,
}

impl Builder {
    fn pt(&self, x: f32, y: f32) -> kurbo::Point {
        kurbo::Point::new(
            self.dx + x as f64 * self.scale,
            self.dy - y as f64 * self.scale,
        )
    }
}

impl ttf_parser::OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = self.pt(x, y);
        self.path.move_to(p);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.pt(x, y);
        self.path.line_to(p);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (a, b) = (self.pt(x1, y1), self.pt(x, y));
        self.path.quad_to(a, b);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, b, c) = (self.pt(x1, y1), self.pt(x2, y2), self.pt(x, y));
        self.path.curve_to(a, b, c);
    }
    fn close(&mut self) {
        self.path.close_path();
    }
}

fn err(code: &str, text: &str, hint: &str) -> UserMessage {
    UserMessage::new(code, text, Some(hint))
}

fn is_wide(c: char) -> bool {
    !c.is_ascii()
}

/// 把文字排成多邊形。`height` 是字級（0.1mm），例如 100 代表 10mm。
pub fn text_to_polygon(
    fonts: &FontSet,
    font_id: &str,
    text: &str,
    height: f64,
    layout: Layout,
) -> Result<TextLayout, UserMessage> {
    if text.trim().is_empty() {
        return Err(err("empty_text", "還沒有輸入文字。", "請先輸入想繡的字。"));
    }
    if height.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return Err(err("bad_height", "字的大小要大於零。", "請把字調大一點。"));
    }
    let primary = fonts
        .index_of(font_id)
        .ok_or_else(|| err("unknown_font", "找不到這個字型。", "請換一個字型再試一次。"))?;

    let mut missing: Vec<char> = Vec::new();
    let mut fallback_used: Vec<char> = Vec::new();
    // 逐段（以換行字元分段）收集字形
    let mut paragraphs: Vec<Vec<Glyph>> = Vec::new();
    for para in text.split('\n') {
        let mut glyphs = Vec::new();
        for ch in para.chars() {
            if ch == '\r' {
                continue;
            }
            let order = std::iter::once(primary).chain((0..fonts.len()).filter(|&i| i != primary));
            let mut found = None;
            for fi in order {
                let face = fonts.face(fi);
                if let Some(id) = face.glyph_index(ch) {
                    let scale = height / face.units_per_em() as f64;
                    let adv = face.glyph_hor_advance(id).unwrap_or(0) as f64 * scale;
                    found = Some((fi, id, adv));
                    break;
                }
            }
            match found {
                Some((fi, id, advance)) => {
                    if fi != primary && !fallback_used.contains(&ch) {
                        fallback_used.push(ch);
                    }
                    glyphs.push(Glyph {
                        ch,
                        font: fi,
                        id,
                        advance,
                    });
                }
                None => {
                    if !missing.contains(&ch) {
                        missing.push(ch);
                    }
                }
            }
        }
        paragraphs.push(glyphs);
    }

    // 換行
    let mut lines: Vec<Vec<&Glyph>> = Vec::new();
    for glyphs in &paragraphs {
        match layout {
            Layout::SingleLine => lines.push(glyphs.iter().collect()),
            Layout::Wrap { max_width } => {
                let mut cur: Vec<&Glyph> = Vec::new();
                let mut width = 0.0;
                for g in glyphs {
                    if width + g.advance > max_width && !cur.is_empty() {
                        // 西文優先在最後一個空白處斷行
                        let split = if !is_wide(g.ch) {
                            cur.iter().rposition(|x| x.ch == ' ').map(|i| i + 1)
                        } else {
                            None
                        };
                        let rest = match split {
                            Some(i) if i < cur.len() => cur.split_off(i),
                            _ => Vec::new(),
                        };
                        lines.push(std::mem::take(&mut cur));
                        width = rest.iter().map(|x| x.advance).sum();
                        cur = rest;
                    }
                    cur.push(g);
                    width += g.advance;
                }
                lines.push(cur);
            }
        }
    }

    // 排版與取輪廓
    let mut rings = Vec::new();
    for (li, line) in lines.iter().enumerate() {
        let trimmed_len = line.iter().rposition(|g| g.ch != ' ').map_or(0, |i| i + 1);
        let line = &line[..trimmed_len];
        let width: f64 = line.iter().map(|g| g.advance).sum();
        let mut pen = -width / 2.0;
        let baseline = li as f64 * height * LINE_HEIGHT;
        for g in line {
            let face = fonts.face(g.font);
            let mut b = Builder {
                path: BezPath::new(),
                scale: height / face.units_per_em() as f64,
                dx: pen,
                dy: baseline,
            };
            if face.outline_glyph(g.id, &mut b).is_some() {
                rings.extend(flatten_path(&b.path, FLATTEN_TOLERANCE));
            }
            pen += g.advance;
        }
    }

    let polygon = Polygon::new(rings);
    let Some((min, max)) = polygon.bounds() else {
        return Err(err(
            "no_glyphs",
            "這些字在這個字型裡都找不到。",
            "請換一個字型，或改用比較常見的字。",
        ));
    };
    let polygon = polygon.translate(-(min[0] + max[0]) / 2.0, -(min[1] + max[1]) / 2.0);
    Ok(TextLayout {
        polygon,
        missing,
        fallback_used,
    })
}
