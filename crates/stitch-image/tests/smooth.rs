mod common;
use common::*;
use stitch_image::smooth::edge_preserving_smooth;
use stitch_image::trace::remove_small_regions;

/// 固定規律的雜訊（不用亂數，結果才可重現）。
fn noise(x: usize, y: usize, amp: i32) -> i32 {
    (((x * 7919 + y * 104729) % 97) as i32 - 48) * amp / 48
}

fn noisy(w: usize, h: usize, base: Rgb, amp: i32) -> Img {
    let mut img = Img::filled(w, h, base);
    for y in 0..h {
        for x in 0..w {
            let n = noise(x, y, amp);
            let c = [0, 1, 2].map(|k| (base[k] as i32 + n).clamp(0, 255) as u8);
            img.set(x, y, c, 255);
        }
    }
    img
}

fn spread(img: &Img, x0: usize, y0: usize, x1: usize, y1: usize) -> f64 {
    let mut vals = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            vals.push(img.rgba[(y * img.w + x) * 4] as f64);
        }
    }
    let m = vals.iter().sum::<f64>() / vals.len() as f64;
    (vals.iter().map(|v| (v - m).powi(2)).sum::<f64>() / vals.len() as f64).sqrt()
}

#[test]
fn 平滑會讓平坦區域的雜訊變小() {
    let img = noisy(40, 40, [120, 90, 80], 30);
    let before = spread(&img, 5, 5, 35, 35);
    let out = edge_preserving_smooth(&img.rgba, 40, 40, 50);
    let after = spread(
        &Img {
            w: 40,
            h: 40,
            rgba: out,
        },
        5,
        5,
        35,
        35,
    );
    assert!(after < before * 0.5, "雜訊 {before:.1} → {after:.1}");
}

#[test]
fn 平滑保留明顯的邊緣() {
    // 左半黑、右半白，各帶雜訊；平滑後邊界兩側仍然一黑一白，不會糊成灰
    let mut img = Img::filled(40, 40, BLACK);
    for y in 0..40 {
        for x in 20..40 {
            img.set(x, y, WHITE, 255);
        }
    }
    let out = edge_preserving_smooth(&img.rgba, 40, 40, 100);
    let at = |x: usize, y: usize| out[(y * 40 + x) * 4];
    assert!(at(19, 20) < 40, "黑的一側變灰了：{}", at(19, 20));
    assert!(at(20, 20) > 215, "白的一側變灰了：{}", at(20, 20));
}

#[test]
fn 平滑不動透明的像素也不被透明的像素影響() {
    let mut img = Img::filled(10, 10, RED);
    img.set(5, 5, [0, 0, 0], 0);
    let out = edge_preserving_smooth(&img.rgba, 10, 10, 80);
    assert_eq!(out[(5 * 10 + 5) * 4 + 3], 0);
    // 紅色像素的顏色不會因為旁邊的透明像素（rgb 是黑）而變暗
    assert_eq!(&out[(5 * 10 + 4) * 4..(5 * 10 + 4) * 4 + 3], &RED);
}

#[test]
fn 強度零時原樣不動() {
    let img = noisy(16, 16, [10, 200, 30], 20);
    assert_eq!(edge_preserving_smooth(&img.rgba, 16, 16, 0), img.rgba);
}

fn labels(w: usize, h: usize, f: impl Fn(usize, usize) -> i16) -> Vec<i16> {
    (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| f(x, y))
        .collect()
}

#[test]
fn 太小的色塊併入周圍最多的顏色() {
    // 全是 0，中間有一小塊 3 像素的 1、一大塊 36 像素的 2
    let mut l = labels(20, 20, |_, _| 0);
    for x in 5..8 {
        l[5 * 20 + x] = 1;
    }
    for y in 10..16 {
        for x in 10..16 {
            l[y * 20 + x] = 2;
        }
    }
    remove_small_regions(&mut l, 20, 20, 10);
    assert!(l.iter().all(|&v| v != 1), "小塊還在");
    assert_eq!(l.iter().filter(|&&v| v == 2).count(), 36, "大塊被動到了");
}

#[test]
fn 太小的色塊鄰著背景時併入背景() {
    // 背景(-1)包著一小塊 0
    let mut l = labels(20, 20, |_, _| -1);
    l[10 * 20 + 10] = 0;
    l[10 * 20 + 11] = 0;
    remove_small_regions(&mut l, 20, 20, 10);
    assert!(l.iter().all(|&v| v == -1));
}

#[test]
fn 門檻是零時不動() {
    let mut l = labels(8, 8, |x, _| if x == 3 { 1 } else { 0 });
    let before = l.clone();
    remove_small_regions(&mut l, 8, 8, 0);
    assert_eq!(l, before);
}
