use stitch_image::trace::{label_polygon, majority_filter};

fn grid(w: usize, h: usize, f: impl Fn(usize, usize) -> i16) -> Vec<i16> {
    (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| f(x, y))
        .collect()
}

#[test]
fn 單一方塊的面積與像素數相同() {
    let labels = grid(20, 20, |x, y| {
        if (5..15).contains(&x) && (5..15).contains(&y) {
            1
        } else {
            0
        }
    });
    let p = label_polygon(&labels, 20, 20, 1);
    assert!((p.area() - 100.0).abs() < 1e-6, "{}", p.area());
    assert_eq!(p.shapes().len(), 1);
    let p0 = label_polygon(&labels, 20, 20, 0);
    assert!((p0.area() - 300.0).abs() < 1e-6, "{}", p0.area());
}

#[test]
fn 環狀區域保留中間的洞() {
    let labels = grid(20, 20, |x, y| {
        let outer = (2..18).contains(&x) && (2..18).contains(&y);
        let inner = (7..13).contains(&x) && (7..13).contains(&y);
        i16::from(outer && !inner)
    });
    let p = label_polygon(&labels, 20, 20, 1);
    assert!((p.area() - (256.0 - 36.0)).abs() < 1e-6, "{}", p.area());
    assert!(!p.contains([10.0, 10.0]));
    assert!(p.contains([3.0, 3.0]));
}

#[test]
fn 分開的兩塊是兩個形狀() {
    let labels = grid(30, 10, |x, _| i16::from(!(8..20).contains(&x)));
    assert_eq!(label_polygon(&labels, 30, 10, 1).shapes().len(), 2);
}

#[test]
fn 不存在的標籤沒有多邊形() {
    let labels = grid(5, 5, |_, _| 0);
    assert!(label_polygon(&labels, 5, 5, 7).is_empty());
}

#[test]
fn 座標是像素的格線座標_y_向下() {
    let labels = grid(10, 10, |x, y| i16::from(x == 3 && y == 4));
    let (min, max) = label_polygon(&labels, 10, 10, 1).bounds().unwrap();
    assert_eq!((min, max), ([3.0, 4.0], [4.0, 5.0]));
}

#[test]
fn 多數決濾波會去掉孤立的雜點() {
    let mut labels = grid(15, 15, |_, _| 0);
    labels[7 * 15 + 7] = 1;
    majority_filter(&mut labels, 15, 15, 1);
    assert_eq!(labels[7 * 15 + 7], 0);
}

#[test]
fn 多數決濾波不會吃掉夠大的區塊() {
    let mut labels = grid(20, 20, |x, y| {
        i16::from((5..15).contains(&x) && (5..15).contains(&y))
    });
    let before = labels.clone();
    majority_filter(&mut labels, 20, 20, 2);
    let changed = labels.iter().zip(&before).filter(|(a, b)| a != b).count();
    assert!(changed <= 8, "大區塊只應在角落有少許變動：{changed}");
}

#[test]
fn 透明的像素不受濾波影響() {
    let mut labels = grid(9, 9, |x, _| if x < 4 { -1 } else { 0 });
    majority_filter(&mut labels, 9, 9, 2);
    assert!(labels.iter().take(4).all(|&l| l == -1));
}
