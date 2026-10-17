use stitch_core::{Stitch, StitchKind, StitchPlan, ThreadColor};
use stitch_formats::dst::{decode_delta, encode_record, Dst};
use stitch_formats::Format;

fn black() -> ThreadColor {
    ThreadColor::new(0, 0, 0, "黑")
}

#[test]
fn 位移編碼的_golden_位元組() {
    // 一般針 b2 = 0x03；跳針 0x83；換色 0xC3；結束 0xF3
    assert_eq!(encode_record(1, 0, 0x03), [0x01, 0x00, 0x03]);
    // DST 的 Y 軸與畫面相反：往下 1 → 檔案中是 -1
    assert_eq!(encode_record(0, 1, 0x03), [0x40, 0x00, 0x03]);
    assert_eq!(encode_record(0, -1, 0x03), [0x80, 0x00, 0x03]);
    assert_eq!(encode_record(121, 0, 0x03), [0x05, 0x05, 0x07]);
    assert_eq!(encode_record(-121, 0, 0x03), [0x0A, 0x0A, 0x0B]);
    assert_eq!(encode_record(0, 0, 0xC3), [0x00, 0x00, 0xC3]);
    assert_eq!(encode_record(0, 0, 0xF3), [0x00, 0x00, 0xF3]);
}

#[test]
fn 位移編碼與解碼對所有合法值互為反函數() {
    for x in -121..=121 {
        for y in [-121, -60, -41, -14, -5, -2, -1, 0, 1, 2, 5, 14, 41, 60, 121] {
            let r = encode_record(x, y, 0x03);
            assert_eq!(decode_delta(r), (x, y), "x={x} y={y}");
        }
    }
}

#[test]
fn 表頭為_512_位元組且欄位正確() {
    let plan = StitchPlan {
        stitches: vec![
            Stitch::stitch(100, 50),
            Stitch::stitch(-30, -20),
            Stitch::color_change(-30, -20),
            Stitch::stitch(0, 0),
            Stitch::end(0, 0),
        ],
        colors: vec![black(), ThreadColor::new(255, 0, 0, "紅")],
    };
    let bytes = Dst.encode(&plan, "Amy").unwrap();
    let header = String::from_utf8_lossy(&bytes[..512]).to_string();
    assert!(header.starts_with("LA:Amy             \r"), "{header:?}");
    assert!(header.contains("CO:  1\r"));
    assert!(header.contains("+X:  100\r"));
    assert!(header.contains("-X:   30\r"));
    // 檔案中的 Y 軸與畫面相反：畫面最低點 50 對應 -Y，畫面最高點 -20 對應 +Y
    assert!(header.contains("+Y:   20\r"));
    assert!(header.contains("-Y:   50\r"));
    assert_eq!(bytes[512 - 1], b' ');
    assert_eq!((bytes.len() - 512) % 3, 0);
    // 最後一筆是結束碼
    assert_eq!(bytes[bytes.len() - 1], 0xF3);
}

#[test]
fn 長位移會被拆成多筆且每筆不超過_121() {
    let plan = StitchPlan {
        stitches: vec![Stitch::stitch(500, 0), Stitch::end(500, 0)],
        colors: vec![black()],
    };
    let bytes = Dst.encode(&plan, "L").unwrap();
    let records = &bytes[512..];
    let n = records.len() / 3;
    assert_eq!(n, 5 + 1); // ceil(500/121) = 5，再加結束
    let mut x = 0;
    for r in records.chunks(3).take(5) {
        let (dx, _) = decode_delta([r[0], r[1], r[2]]);
        assert!(dx.abs() <= 121);
        x += dx;
    }
    assert_eq!(x, 500);
}

#[test]
fn 往返後針點等價() {
    let plan = StitchPlan {
        stitches: vec![
            Stitch::stitch(10, 20),
            Stitch::jump(200, 20),
            Stitch::stitch(205, 25),
            Stitch::color_change(205, 25),
            Stitch::stitch(0, 0),
            Stitch::end(0, 0),
        ],
        colors: vec![black(), ThreadColor::new(255, 0, 0, "紅")],
    };
    let bytes = Dst.encode(&plan, "RT").unwrap();
    let back = Dst.decode(&bytes).unwrap();
    // 200-10=190 的跳針被拆為兩段，其餘不變
    let kinds: Vec<_> = back.stitches.iter().map(|s| s.kind).collect();
    assert_eq!(back.stitches.last().unwrap().kind, StitchKind::End);
    assert_eq!(back.color_change_count(), 1);
    assert_eq!(back.colors.len(), 2);
    let end_xy: Vec<_> = back.stitches.iter().map(|s| (s.x, s.y)).collect();
    assert!(end_xy.contains(&(200, 20)));
    assert!(end_xy.contains(&(205, 25)));
    assert!(kinds.contains(&StitchKind::Jump));
}

#[test]
fn 剪線以連續三筆零位移跳針表示() {
    let plan = StitchPlan {
        stitches: vec![Stitch::stitch(5, 5), Stitch::trim(5, 5), Stitch::end(5, 5)],
        colors: vec![black()],
    };
    let bytes = Dst.encode(&plan, "T").unwrap();
    let recs: Vec<_> = bytes[512..].chunks(3).collect();
    assert_eq!(recs.len(), 1 + 3 + 1);
    for r in &recs[1..4] {
        assert_eq!(r[2], 0x83);
        assert_eq!(decode_delta([r[0], r[1], r[2]]), (0, 0));
    }
}

#[test]
fn 標籤超過_16_字元會被截斷且非_ascii_以底線取代() {
    let plan = StitchPlan {
        stitches: vec![Stitch::stitch(1, 1), Stitch::end(1, 1)],
        colors: vec![black()],
    };
    let bytes = Dst.encode(&plan, "王小明ABCDEFGHIJKLMNOPQRS").unwrap();
    let header = String::from_utf8_lossy(&bytes[..20]).to_string();
    assert_eq!(header, "LA:___ABCDEFGHIJKLM\r");
}

#[test]
fn 過短的資料解碼失敗() {
    assert!(Dst.decode(&[0u8; 10]).is_err());
}
