use stitch_gen::min_size::{min_scale_needed, MinSizeParams};
use stitch_geom::Polygon;

fn rect(w: f64, h: f64) -> Polygon {
    Polygon::new(vec![vec![[0.0, 0.0], [w, 0.0], [w, h], [0.0, h]]])
}

#[test]
fn 預設值與規格一致() {
    let p = MinSizeParams::default();
    assert_eq!(p.min_text_height, 40.0); // 4mm
    assert_eq!(p.min_stroke_width, 8.0); // 0.8mm
}

#[test]
fn 字太小要放大到最小字高() {
    // 粗矩形：筆畫沒有問題，只受字高限制
    let s = min_scale_needed(&rect(100.0, 30.0), 30.0, &MinSizeParams::default());
    assert!((s - 40.0 / 30.0).abs() < 0.02, "s={s}");
}

#[test]
fn 夠大的粗筆畫不需要放大() {
    let s = min_scale_needed(&rect(100.0, 30.0), 100.0, &MinSizeParams::default());
    assert!(s <= 1.0, "s={s}");
}

#[test]
fn 筆畫太細要放大到筆畫夠寬() {
    // 寬 4 的細條：低於 8，至少要放大一倍多
    let s = min_scale_needed(&rect(200.0, 4.0), 100.0, &MinSizeParams::default());
    assert!(s > 1.9 && s < 3.0, "s={s}");
}

#[test]
fn 筆畫細到放大三倍也不夠時回傳上限() {
    let s = min_scale_needed(&rect(200.0, 1.0), 100.0, &MinSizeParams::default());
    assert!((s - 3.0).abs() < 1e-9, "s={s}");
}

use stitch_gen::min_size::{enlarge_needed, shrink_limit};

#[test]
fn 放大倍率不會小於一() {
    let p = MinSizeParams::default();
    assert_eq!(enlarge_needed(&rect(100.0, 30.0), 100.0, &p), 1.0);
    let by_height = enlarge_needed(&rect(100.0, 30.0), 30.0, &p);
    assert!((by_height - 40.0 / 30.0).abs() < 1e-9, "{by_height}");
}

#[test]
fn 筆畫太細時放大倍率介於一與三之間() {
    let s = enlarge_needed(&rect(200.0, 4.0), 100.0, &MinSizeParams::default());
    assert!(s > 1.9 && s < 3.0, "s={s}");
    let capped = enlarge_needed(&rect(200.0, 1.0), 100.0, &MinSizeParams::default());
    assert!((capped - 3.0).abs() < 1e-9);
}

#[test]
fn 字高下限比筆畫下限更嚴時以字高為準() {
    // 太小的字：高度要求（4mm）比筆畫（粗矩形）更嚴
    let s = enlarge_needed(&rect(100.0, 30.0), 10.0, &MinSizeParams::default());
    assert!((s - 4.0).abs() < 1e-9, "s={s}");
}

#[test]
fn 縮小下限_受字高限制() {
    // 筆畫很粗、字級 100：字高下限（40）是限制，最多縮到 0.4 倍以上
    let s = shrink_limit(&rect(300.0, 120.0), 1.0, 100.0, &MinSizeParams::default());
    assert!((s - 0.4).abs() < 0.02, "s={s}");
}

#[test]
fn 縮小下限的性質_在下限處夠大_略低於下限就不夠() {
    // 判斷「夠不夠大」的標準與 enlarge_needed 相同，所以拿它當裁判：
    // 縮到下限時不需要再放大；再縮 5% 就需要放大。
    let p = MinSizeParams::default();
    for (w, h, size) in [
        (100.0, 30.0, 100.0),
        (300.0, 20.0, 200.0),
        (80.0, 80.0, 500.0),
        (120.0, 40.0, 1000.0),
    ] {
        let poly = rect(w, h);
        // 前提：原大小就已經夠大（shrink_limit 是從「夠大」往下找）
        assert_eq!(
            enlarge_needed(&poly, size, &p),
            1.0,
            "{w}x{h} 原大小就不夠大"
        );
        let limit = shrink_limit(&poly, 1.0, size, &p);
        assert!(limit > 0.0 && limit <= 1.0, "{w}x{h}: limit={limit}");
        let at = enlarge_needed(&poly.scale(limit), size * limit, &p);
        assert!(at <= 1.0 + 1e-6, "{w}x{h}: 下限 {limit} 處仍需放大到 {at}");
        let below = enlarge_needed(&poly.scale(limit * 0.95), size * limit * 0.95, &p);
        assert!(
            below > 1.0 + 1e-6,
            "{w}x{h}: 再縮 5% 應該不夠大，卻只需 {below}"
        );
    }
}

#[test]
fn 縮小下限不會比目前的倍率更大() {
    let p = MinSizeParams::default();
    let poly = rect(200.0, 60.0);
    // 從 2.0 往下找：下限一定 ≤ 2.0
    assert!(shrink_limit(&poly, 2.0, 100.0, &p) <= 2.0);
    // 已經在字高下限上就不能再縮
    assert_eq!(shrink_limit(&poly, 1.0, 40.0, &p), 1.0);
}
