//! Melco EXP：一般針為 2 位元組有號位移；`0x80` 開頭為控制碼。檔尾沒有明確結束記錄。

use crate::format::placeholder_colors;
use crate::split::split_move;
use crate::{Format, FormatError};
use stitch_core::{Stitch, StitchKind, StitchPlan};

pub struct Exp;

const MAX_STEP: i32 = 127;

fn pair(dx: i32, dy: i32) -> [u8; 2] {
    // EXP 的 Y 軸向上，畫面向下為正，因此反轉
    [(dx as i8) as u8, ((-dy) as i8) as u8]
}

impl Format for Exp {
    fn id(&self) -> &'static str {
        "exp"
    }
    fn extension(&self) -> &'static str {
        "EXP"
    }
    fn label(&self) -> &'static str {
        "通用格式 B"
    }
    fn description(&self) -> &'static str {
        "另一種常見的通用格式（副檔名 .EXP）"
    }
    fn max_step(&self) -> i32 {
        MAX_STEP
    }

    fn encode(&self, plan: &StitchPlan, _label: &str) -> Result<Vec<u8>, FormatError> {
        let mut out = Vec::new();
        let (mut cx, mut cy) = (0i32, 0i32);
        for s in &plan.stitches {
            match s.kind {
                StitchKind::Stitch => {
                    for (dx, dy) in split_move(s.x - cx, s.y - cy, MAX_STEP) {
                        out.extend_from_slice(&pair(dx, dy));
                        cx += dx;
                        cy += dy;
                    }
                }
                StitchKind::Jump => {
                    for (dx, dy) in split_move(s.x - cx, s.y - cy, MAX_STEP) {
                        out.extend_from_slice(&[0x80, 0x04]);
                        out.extend_from_slice(&pair(dx, dy));
                        cx += dx;
                        cy += dy;
                    }
                }
                StitchKind::Trim => out.extend_from_slice(&[0x80, 0x04, 0, 0]),
                StitchKind::ColorChange => out.extend_from_slice(&[0x80, 0x01, 0, 0]),
                StitchKind::End => {}
            }
        }
        Ok(out)
    }

    fn decode(&self, bytes: &[u8]) -> Result<StitchPlan, FormatError> {
        let (mut x, mut y) = (0i32, 0i32);
        let mut stitches = Vec::new();
        let mut changes = 0usize;
        let mut i = 0;
        while i < bytes.len() {
            if i + 1 >= bytes.len() {
                return Err(FormatError::Malformed("EXP 資料在中途結束".into()));
            }
            let (b0, b1) = (bytes[i], bytes[i + 1]);
            i += 2;
            if b0 == 0x80 {
                if i + 1 >= bytes.len() {
                    return Err(FormatError::Malformed("EXP 控制碼缺少後續資料".into()));
                }
                let (c0, c1) = (bytes[i] as i8 as i32, bytes[i + 1] as i8 as i32);
                i += 2;
                if b1 & 0x01 != 0 {
                    changes += 1;
                    stitches.push(Stitch::color_change(x, y));
                } else if c0 == 0 && c1 == 0 {
                    stitches.push(Stitch::trim(x, y));
                } else {
                    x += c0;
                    y -= c1;
                    stitches.push(Stitch::jump(x, y));
                }
            } else {
                x += b0 as i8 as i32;
                y -= b1 as i8 as i32;
                stitches.push(Stitch::stitch(x, y));
            }
        }
        stitches.push(Stitch::end(x, y));
        Ok(StitchPlan {
            stitches,
            colors: placeholder_colors(changes),
        })
    }
}
