mod common;
use common::*;
use stitch_image::saliency::importance_map;
use stitch_image::trace::remove_small_regions_by;

fn mean_in(map: &[f64], w: usize, x0: usize, y0: usize, x1: usize, y1: usize) -> f64 {
    let mut s = 0.0;
    for y in y0..y1 {
        for x in x0..x1 {
            s += map[y * w + x];
        }
    }
    s / ((x1 - x0) * (y1 - y0)) as f64
}

#[test]
fn 重要性在零到一之間_全白的圖沒有重點() {
    let img = Img::filled(60, 60, WHITE);
    let m = importance_map(&img.rgba, 60, 60);
    assert_eq!(m.len(), 3600);
    assert!(m.iter().all(|&v| (0.0..=1.0).contains(&v)));
    assert!(m.iter().all(|&v| v < 0.05), "全白的圖不該有重點");
}

#[test]
fn 畫面中央對比強的比角落對比弱的重要() {
    let mut img = Img::filled(100, 100, WHITE);
    img.rect(45, 45, 55, 55, BLACK); // 中央的深色塊
    img.rect(5, 5, 15, 15, [250, 235, 180]); // 角落、很淺的塊
    let m = importance_map(&img.rgba, 100, 100);
    let (center, corner) = (
        mean_in(&m, 100, 45, 45, 55, 55),
        mean_in(&m, 100, 5, 5, 15, 15),
    );
    assert!(center > corner * 3.0, "中央 {center} 角落 {corner}");
    assert!(center > 0.5, "中央的重點應明顯：{center}");
}

#[test]
fn 同樣的塊放在角落比放在中央不重要() {
    // 排除顏色因素：兩塊顏色相同、大小相同，只有位置不同
    let mut img = Img::filled(100, 100, WHITE);
    img.rect(45, 45, 55, 55, RED);
    img.rect(85, 85, 95, 95, RED);
    let m = importance_map(&img.rgba, 100, 100);
    assert!(mean_in(&m, 100, 45, 45, 55, 55) > mean_in(&m, 100, 85, 85, 95, 95));
}

#[test]
fn 全透明的地方沒有重要性() {
    let mut img = Img::filled(40, 40, WHITE);
    img.rect(15, 15, 25, 25, BLACK);
    for y in 0..10 {
        for x in 0..10 {
            img.set(x, y, [0, 0, 0], 0);
        }
    }
    let m = importance_map(&img.rgba, 40, 40);
    assert_eq!(mean_in(&m, 40, 0, 0, 10, 10), 0.0);
}

#[test]
fn 小色塊的刪除門檻可以依位置不同() {
    // 兩個同樣大小（36 像素）的色塊：左邊的門檻高、右邊的低
    let (w, h) = (40, 20);
    let mut labels = vec![-1i16; w * h];
    for y in 5..11 {
        for x in 3..9 {
            labels[y * w + x] = 0;
        }
        for x in 30..36 {
            labels[y * w + x] = 0;
        }
    }
    remove_small_regions_by(&mut labels, w, h, |px| if px[0] % w < 20 { 50 } else { 20 });
    assert_eq!(labels[7 * w + 5], -1, "左邊的門檻高，應被刪掉");
    assert_eq!(labels[7 * w + 32], 0, "右邊的門檻低，應保留");
}
