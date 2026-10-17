mod common;
use common::*;
use stitch_image::quantize::{quantize, suggest_color_count};

#[test]
fn 三色加背景分成四群且中心接近原色() {
    let img = logo(64);
    let q = quantize(&img.rgba, img.w, img.h, 4).unwrap();
    assert_eq!(q.centers.len(), 4);
    for want in [WHITE, RED, BLUE, GREEN] {
        let best = q
            .centers
            .iter()
            .map(|&c| dist(c, want))
            .fold(f64::MAX, f64::min);
        assert!(best < 12.0, "找不到接近 {want:?} 的群，最近距離 {best}");
    }
    assert!(q.mean_error < 3.0, "平坦色塊的誤差應很小：{}", q.mean_error);
}

#[test]
fn 每個像素的標籤對應到最接近的顏色() {
    let img = logo(64);
    let q = quantize(&img.rgba, img.w, img.h, 4).unwrap();
    let label_at = |x: usize, y: usize| q.labels[y * img.w + x];
    let red_label = label_at(10, 10);
    assert!(dist(q.centers[red_label as usize], RED) < 12.0);
    let white_label = label_at(0, 0);
    assert!(dist(q.centers[white_label as usize], WHITE) < 12.0);
    assert_ne!(red_label, white_label);
}

#[test]
fn 結果是固定的_不受隨機影響() {
    let img = logo(48);
    let a = quantize(&img.rgba, img.w, img.h, 4).unwrap();
    let b = quantize(&img.rgba, img.w, img.h, 4).unwrap();
    assert_eq!(a.labels, b.labels);
    assert_eq!(a.centers, b.centers);
}

#[test]
fn 顏色數必須在二到十二之間() {
    let img = logo(16);
    for bad in [0usize, 1, 13, 100] {
        let e = quantize(&img.rgba, img.w, img.h, bad).unwrap_err();
        assert_eq!(e.code, "bad_color_count", "k={bad}");
    }
    for ok in [2usize, 12] {
        assert!(quantize(&img.rgba, img.w, img.h, ok).is_ok(), "k={ok}");
    }
}

#[test]
fn 要求的顏色比圖片實際的多時不會當機() {
    let img = Img::filled(8, 8, RED);
    let q = quantize(&img.rgba, img.w, img.h, 6).unwrap();
    assert!(!q.centers.is_empty() && q.centers.len() <= 6);
}

#[test]
fn 透明的像素標記為_負一() {
    let mut img = Img::filled(8, 8, RED);
    img.set(0, 0, RED, 0);
    img.set(1, 0, RED, 100);
    let q = quantize(&img.rgba, img.w, img.h, 2).unwrap();
    assert_eq!(q.labels[0], -1);
    assert_eq!(q.labels[1], -1); // 半透明（alpha < 128）也視為透明
    assert!(q.labels[2] >= 0);
}

#[test]
fn 全透明的圖回報白話錯誤() {
    let mut img = Img::filled(4, 4, RED);
    for y in 0..4 {
        for x in 0..4 {
            img.set(x, y, RED, 0);
        }
    }
    let e = quantize(&img.rgba, img.w, img.h, 3).unwrap_err();
    assert_eq!(e.code, "empty_image");
}

#[test]
fn 資料長度與尺寸不符時回報錯誤() {
    let e = quantize(&[0u8; 10], 4, 4, 3).unwrap_err();
    assert_eq!(e.code, "bad_image");
}

#[test]
fn 建議的顏色數_簡單圖案偏少_在範圍內() {
    let img = logo(64);
    let k = suggest_color_count(&img.rgba, img.w, img.h);
    assert!((2..=8).contains(&k), "k={k}");
    assert!(k >= 3, "三色加背景至少要 3：{k}");
    let flat = Img::filled(16, 16, RED);
    assert_eq!(suggest_color_count(&flat.rgba, flat.w, flat.h), 2);
}
