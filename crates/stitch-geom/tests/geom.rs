use kurbo::{BezPath, Point as KPoint};
use stitch_geom::{flatten_path, spans_at, Polygon};

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

fn square_cw(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
    let mut r = square(x0, y0, x1, y1);
    r.reverse();
    r
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn 面積與邊界() {
    let p = Polygon::new(vec![square(0.0, 0.0, 100.0, 50.0)]);
    assert!(close(p.area(), 5000.0));
    let (min, max) = p.bounds().unwrap();
    assert_eq!((min, max), ([0.0, 0.0], [100.0, 50.0]));
    assert!(Polygon::new(vec![]).bounds().is_none());
}

#[test]
fn 有洞的面積是外框減洞() {
    let p = Polygon::new(vec![
        square(0.0, 0.0, 100.0, 100.0),
        square_cw(25.0, 25.0, 75.0, 75.0),
    ]);
    assert!(close(p.area(), 10000.0 - 2500.0));
}

#[test]
fn 點在多邊形內判斷會避開洞() {
    let p = Polygon::new(vec![
        square(0.0, 0.0, 100.0, 100.0),
        square_cw(25.0, 25.0, 75.0, 75.0),
    ]);
    assert!(p.contains([10.0, 10.0]));
    assert!(!p.contains([50.0, 50.0]));
    assert!(!p.contains([150.0, 50.0]));
}

#[test]
fn 掃描線求交_單一矩形() {
    let p = Polygon::new(vec![square(0.0, 0.0, 100.0, 100.0)]);
    assert_eq!(spans_at(&p, 50.0), vec![(0.0, 100.0)]);
    assert!(spans_at(&p, 150.0).is_empty());
    assert!(spans_at(&p, -1.0).is_empty());
}

#[test]
fn 掃描線穿過洞時分成兩段() {
    let p = Polygon::new(vec![
        square(0.0, 0.0, 100.0, 100.0),
        square_cw(25.0, 25.0, 75.0, 75.0),
    ]);
    assert_eq!(spans_at(&p, 50.0), vec![(0.0, 25.0), (75.0, 100.0)]);
    assert_eq!(spans_at(&p, 10.0), vec![(0.0, 100.0)]);
}

#[test]
fn 同向重疊的兩個矩形合併成一段() {
    let p = Polygon::new(vec![
        square(0.0, 0.0, 60.0, 100.0),
        square(40.0, 0.0, 100.0, 100.0),
    ]);
    assert_eq!(spans_at(&p, 50.0), vec![(0.0, 100.0)]);
}

#[test]
fn 兩個分開的矩形各有一段() {
    let p = Polygon::new(vec![
        square(0.0, 0.0, 30.0, 100.0),
        square(70.0, 0.0, 100.0, 100.0),
    ]);
    assert_eq!(spans_at(&p, 50.0), vec![(0.0, 30.0), (70.0, 100.0)]);
}

#[test]
fn 水平邊不會造成重複或遺漏() {
    // 掃描線剛好落在上下邊界：下邊界（含）上邊界（不含），避免相鄰多邊形重複計算
    let p = Polygon::new(vec![square(0.0, 0.0, 100.0, 100.0)]);
    assert_eq!(spans_at(&p, 0.0), vec![(0.0, 100.0)]);
    assert!(spans_at(&p, 100.0).is_empty());
}

#[test]
fn 曲線展平保留端點且點足夠貼近曲線() {
    let mut path = BezPath::new();
    path.move_to(KPoint::new(0.0, 0.0));
    path.quad_to(KPoint::new(50.0, 100.0), KPoint::new(100.0, 0.0));
    path.close_path();
    let rings = flatten_path(&path, 0.1);
    assert_eq!(rings.len(), 1);
    let ring = &rings[0];
    assert!(ring.len() > 4);
    assert_eq!(ring[0], [0.0, 0.0]);
    assert!(ring
        .iter()
        .any(|p| (p[0] - 100.0).abs() < 1e-9 && p[1].abs() < 1e-9));
    // 曲線在 t=0.5 的位置是 (50, 50)；展平後的點應有接近它的
    assert!(ring
        .iter()
        .any(|p| (p[0] - 50.0).abs() < 1.0 && (p[1] - 50.0).abs() < 1.0));
}

#[test]
fn 多個子路徑各自成環() {
    let mut path = BezPath::new();
    path.move_to(KPoint::new(0.0, 0.0));
    path.line_to(KPoint::new(10.0, 0.0));
    path.line_to(KPoint::new(10.0, 10.0));
    path.close_path();
    path.move_to(KPoint::new(20.0, 0.0));
    path.line_to(KPoint::new(30.0, 0.0));
    path.line_to(KPoint::new(30.0, 10.0));
    path.close_path();
    assert_eq!(flatten_path(&path, 0.1).len(), 2);
}

#[test]
fn 內縮會讓邊界各縮小() {
    let p = Polygon::new(vec![square(0.0, 0.0, 100.0, 100.0)]);
    let inner = p.offset(-10.0);
    let (min, max) = inner.bounds().unwrap();
    assert!(close(min[0], 10.0) && close(min[1], 10.0));
    assert!(close(max[0], 90.0) && close(max[1], 90.0));
}

#[test]
fn 外擴會讓邊界各增大() {
    let p = Polygon::new(vec![square(0.0, 0.0, 100.0, 100.0)]);
    let outer = p.offset(10.0);
    let (min, max) = outer.bounds().unwrap();
    assert!(close(min[0], -10.0) && close(max[0], 110.0));
}

#[test]
fn 內縮過多會變成空() {
    let p = Polygon::new(vec![square(0.0, 0.0, 10.0, 10.0)]);
    assert!(p.offset(-6.0).rings.is_empty());
}

#[test]
fn 有洞的多邊形內縮後洞會變大() {
    let p = Polygon::new(vec![
        square(0.0, 0.0, 100.0, 100.0),
        square_cw(40.0, 40.0, 60.0, 60.0),
    ]);
    let inner = p.offset(-5.0);
    assert!(!inner.contains([50.0, 50.0]));
    assert!(!inner.contains([37.0, 50.0])); // 原本在實心內，現在落在被撐大的洞裡
    assert!(inner.contains([20.0, 20.0]));
}

#[test]
fn 分成互不相連的形狀_各帶自己的洞() {
    // 回字形：外框＋洞＋洞中的島，另有一個分開的小方塊
    let p = Polygon::new(vec![
        square(0.0, 0.0, 100.0, 100.0),
        square_cw(20.0, 20.0, 80.0, 80.0),
        square(40.0, 40.0, 60.0, 60.0),
        square(200.0, 0.0, 230.0, 30.0),
    ]);
    let shapes = p.shapes();
    assert_eq!(shapes.len(), 3);
    let with_hole = shapes.iter().filter(|s| s.rings.len() == 2).count();
    assert_eq!(with_hole, 1);
    let total: f64 = shapes.iter().map(|s| s.area()).sum();
    assert!(close(total, p.area()));
}

#[test]
fn 空多邊形沒有形狀() {
    assert!(Polygon::default().shapes().is_empty());
}
