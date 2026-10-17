use stitch_core::{Stitch, StitchKind, StitchPlan, ThreadColor};
use stitch_formats::exp::Exp;
use stitch_formats::Format;

fn black() -> ThreadColor {
    ThreadColor::new(0, 0, 0, "黑")
}

#[test]
fn 一般針為兩位元組的有號位移() {
    let plan = StitchPlan {
        stitches: vec![
            Stitch::stitch(3, 4),
            Stitch::stitch(1, 9),
            Stitch::end(1, 9),
        ],
        colors: vec![black()],
    };
    let bytes = Exp.encode(&plan, "x").unwrap();
    // 第一筆 (3,4)：EXP 的 Y 軸向上，畫面向下 4 → 檔案 -4
    assert_eq!(&bytes[0..2], &[3u8, (-4i8) as u8]);
    // 第二筆 (-2, +5)：x 變化 -2，畫面向下 5 → 檔案 -5
    assert_eq!(&bytes[2..4], &[(-2i8) as u8, (-5i8) as u8]);
}

#[test]
fn 跳針與換色使用_0x80_控制碼() {
    let plan = StitchPlan {
        stitches: vec![
            Stitch::stitch(0, 0),
            Stitch::jump(10, 0),
            Stitch::color_change(10, 0),
            Stitch::stitch(10, 0),
            Stitch::end(10, 0),
        ],
        colors: vec![black(), ThreadColor::new(255, 0, 0, "紅")],
    };
    let bytes = Exp.encode(&plan, "x").unwrap();
    // 起點 (0,0) 的第一針位移為 0，不輸出
    assert_eq!(&bytes[0..4], &[0x80, 0x04, 10u8, 0u8]);
    assert_eq!(&bytes[4..8], &[0x80, 0x01, 0x00, 0x00]);
    assert_eq!(bytes.len(), 8);
}

#[test]
fn 長位移拆成不超過_127_的多段() {
    let plan = StitchPlan {
        stitches: vec![Stitch::stitch(400, 0), Stitch::end(400, 0)],
        colors: vec![black()],
    };
    let bytes = Exp.encode(&plan, "x").unwrap();
    assert_eq!(bytes.len() / 2, 4); // ceil(400/127) = 4
    let sum: i32 = bytes.chunks(2).map(|c| c[0] as i8 as i32).sum();
    assert_eq!(sum, 400);
    assert!(bytes.chunks(2).all(|c| (c[0] as i8 as i32).abs() <= 127));
}

#[test]
fn 往返後終點與換色數一致() {
    let plan = StitchPlan {
        stitches: vec![
            Stitch::stitch(10, 10),
            Stitch::stitch(60, -20),
            Stitch::color_change(60, -20),
            Stitch::stitch(0, 0),
            Stitch::end(0, 0),
        ],
        colors: vec![black(), ThreadColor::new(255, 0, 0, "紅")],
    };
    let bytes = Exp.encode(&plan, "x").unwrap();
    let back = Exp.decode(&bytes).unwrap();
    assert_eq!(back.color_change_count(), 1);
    assert_eq!(back.colors.len(), 2);
    assert_eq!(back.stitches.last().unwrap().kind, StitchKind::End);
    let pts: Vec<_> = back.stitches.iter().map(|s| (s.x, s.y)).collect();
    assert!(pts.contains(&(10, 10)));
    assert!(pts.contains(&(60, -20)));
    assert_eq!(*pts.last().unwrap(), (0, 0));
}

#[test]
fn 剪線以跳針控制碼加零位移表示() {
    let plan = StitchPlan {
        stitches: vec![Stitch::stitch(5, 0), Stitch::trim(5, 0), Stitch::end(5, 0)],
        colors: vec![black()],
    };
    let bytes = Exp.encode(&plan, "x").unwrap();
    assert_eq!(&bytes[2..6], &[0x80, 0x04, 0, 0]);
}
