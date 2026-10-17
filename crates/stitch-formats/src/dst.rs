//! Tajima DST：512 位元組表頭，後接每筆 3 位元組的平衡三進位位移記錄。

use crate::format::placeholder_colors;
use crate::split::split_move;
use crate::{Format, FormatError};
use stitch_core::{Stitch, StitchKind, StitchPlan};

pub struct Dst;

const HEADER_LEN: usize = 512;
const MAX_STEP: i32 = 121;
const FLAG_STITCH: u8 = 0x03;
const FLAG_JUMP: u8 = 0x83;
const FLAG_COLOR: u8 = 0xC3;
const FLAG_END: u8 = 0xF3;

/// 把位移編成 3 位元組。`dy` 為畫面方向（向下為正），檔案內的 Y 軸相反，這裡負責反轉。
pub fn encode_record(dx: i32, dy: i32, flag: u8) -> [u8; 3] {
    let (mut x, mut y) = (dx, -dy);
    let (mut b0, mut b1, mut b2) = (0u8, 0u8, flag);
    if x > 40 {
        b2 |= 0x04;
        x -= 81;
    }
    if x < -40 {
        b2 |= 0x08;
        x += 81;
    }
    if x > 13 {
        b1 |= 0x04;
        x -= 27;
    }
    if x < -13 {
        b1 |= 0x08;
        x += 27;
    }
    if x > 4 {
        b0 |= 0x04;
        x -= 9;
    }
    if x < -4 {
        b0 |= 0x08;
        x += 9;
    }
    if x > 1 {
        b1 |= 0x01;
        x -= 3;
    }
    if x < -1 {
        b1 |= 0x02;
        x += 3;
    }
    if x > 0 {
        b0 |= 0x01;
    }
    if x < 0 {
        b0 |= 0x02;
    }
    if y > 40 {
        b2 |= 0x20;
        y -= 81;
    }
    if y < -40 {
        b2 |= 0x10;
        y += 81;
    }
    if y > 13 {
        b1 |= 0x20;
        y -= 27;
    }
    if y < -13 {
        b1 |= 0x10;
        y += 27;
    }
    if y > 4 {
        b0 |= 0x20;
        y -= 9;
    }
    if y < -4 {
        b0 |= 0x10;
        y += 9;
    }
    if y > 1 {
        b1 |= 0x80;
        y -= 3;
    }
    if y < -1 {
        b1 |= 0x40;
        y += 3;
    }
    if y > 0 {
        b0 |= 0x80;
    }
    if y < 0 {
        b0 |= 0x40;
    }
    [b0, b1, b2]
}

fn bit(b: u8, mask: u8) -> i32 {
    (b & mask != 0) as i32
}

/// 解出一筆記錄的位移（畫面方向）。
pub fn decode_delta(r: [u8; 3]) -> (i32, i32) {
    let [b0, b1, b2] = r;
    let x = bit(b0, 0x01) - bit(b0, 0x02)
        + 9 * (bit(b0, 0x04) - bit(b0, 0x08))
        + 3 * (bit(b1, 0x01) - bit(b1, 0x02))
        + 27 * (bit(b1, 0x04) - bit(b1, 0x08))
        + 81 * (bit(b2, 0x04) - bit(b2, 0x08));
    let y = bit(b0, 0x80) - bit(b0, 0x40)
        + 9 * (bit(b0, 0x20) - bit(b0, 0x10))
        + 3 * (bit(b1, 0x80) - bit(b1, 0x40))
        + 27 * (bit(b1, 0x20) - bit(b1, 0x10))
        + 81 * (bit(b2, 0x20) - bit(b2, 0x10));
    (x, -y)
}

fn label16(label: &str) -> String {
    let mut s: String = label
        .chars()
        .map(|c| {
            if c.is_ascii() && !c.is_ascii_control() {
                c
            } else {
                '_'
            }
        })
        .collect();
    s.truncate(16);
    format!("{s:<16}")
}

impl Format for Dst {
    fn id(&self) -> &'static str {
        "dst"
    }
    fn extension(&self) -> &'static str {
        "DST"
    }
    fn label(&self) -> &'static str {
        "通用格式 A"
    }
    fn description(&self) -> &'static str {
        "大多數繡機都讀得到的通用格式（副檔名 .DST）"
    }
    fn max_step(&self) -> i32 {
        MAX_STEP
    }

    fn encode(&self, plan: &StitchPlan, label: &str) -> Result<Vec<u8>, FormatError> {
        let mut records: Vec<[u8; 3]> = Vec::new();
        let (mut cx, mut cy) = (0i32, 0i32);
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (0i32, 0i32, 0i32, 0i32);
        let mut color_changes = 0usize;

        for s in &plan.stitches {
            match s.kind {
                StitchKind::Stitch | StitchKind::Jump => {
                    let flag = if s.kind == StitchKind::Stitch {
                        FLAG_STITCH
                    } else {
                        FLAG_JUMP
                    };
                    for (dx, dy) in split_move(s.x - cx, s.y - cy, MAX_STEP) {
                        records.push(encode_record(dx, dy, flag));
                        cx += dx;
                        cy += dy;
                        min_x = min_x.min(cx);
                        max_x = max_x.max(cx);
                        min_y = min_y.min(cy);
                        max_y = max_y.max(cy);
                    }
                }
                StitchKind::Trim => {
                    for _ in 0..3 {
                        records.push(encode_record(0, 0, FLAG_JUMP));
                    }
                }
                StitchKind::ColorChange => {
                    color_changes += 1;
                    records.push(encode_record(0, 0, FLAG_COLOR));
                }
                StitchKind::End => records.push(encode_record(0, 0, FLAG_END)),
            }
        }

        let header = format!(
            "LA:{}\rST:{:7}\rCO:{:3}\r+X:{:5}\r-X:{:5}\r+Y:{:5}\r-Y:{:5}\rAX:+{:5}\rAY:+{:5}\rMX:+{:5}\rMY:+{:5}\rPD:******\r",
            label16(label),
            records.len(),
            color_changes,
            max_x,
            -min_x,
            // 檔案的 Y 軸與畫面相反
            -min_y,
            max_y,
            0,
            0,
            0,
            0
        );
        let mut out = header.into_bytes();
        out.push(0x1A);
        out.resize(HEADER_LEN, b' ');
        for r in records {
            out.extend_from_slice(&r);
        }
        Ok(out)
    }

    fn decode(&self, bytes: &[u8]) -> Result<StitchPlan, FormatError> {
        if bytes.len() < HEADER_LEN {
            return Err(FormatError::Malformed("DST 表頭不完整".into()));
        }
        let body = &bytes[HEADER_LEN..];
        if !body.len().is_multiple_of(3) {
            return Err(FormatError::Malformed("DST 記錄長度不是 3 的倍數".into()));
        }
        let (mut x, mut y) = (0i32, 0i32);
        let mut stitches = Vec::new();
        let mut changes = 0usize;
        for r in body.chunks(3) {
            let rec = [r[0], r[1], r[2]];
            let (dx, dy) = decode_delta(rec);
            x += dx;
            y += dy;
            if rec[2] & 0xF3 == FLAG_END {
                stitches.push(Stitch::end(x, y));
                break;
            } else if rec[2] & 0xC3 == FLAG_COLOR {
                changes += 1;
                stitches.push(Stitch::color_change(x, y));
            } else if rec[2] & 0x80 != 0 {
                stitches.push(Stitch::jump(x, y));
            } else {
                stitches.push(Stitch::stitch(x, y));
            }
        }
        if stitches.last().map(|s| s.kind) != Some(StitchKind::End) {
            stitches.push(Stitch::end(x, y));
        }
        Ok(StitchPlan {
            stitches,
            colors: placeholder_colors(changes),
        })
    }
}
