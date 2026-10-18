use stitch_gen::stroke::{
    centerlines, classify, plan_stroke, principal_angle, run_stitch, satin, StrokeKind, SATIN_STEP,
};
use stitch_geom::Polygon;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Polygon {
    Polygon::new(vec![vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]])
}

fn l_shape() -> Polygon {
    // 兩條 20 寬的臂：橫的 0..200、直的 0..120
    Polygon::new(vec![vec![
        [0.0, 0.0],
        [200.0, 0.0],
        [200.0, 20.0],
        [20.0, 20.0],
        [20.0, 120.0],
        [0.0, 120.0],
    ]])
}

#[test]
fn 細長的線走平針_適中的走緞面_寬的和方塊走填充() {
    assert_eq!(classify(&rect(0.0, 0.0, 400.0, 6.0)), StrokeKind::Run);
    assert_eq!(classify(&rect(0.0, 0.0, 400.0, 25.0)), StrokeKind::Satin);
    assert_eq!(classify(&rect(0.0, 0.0, 400.0, 60.0)), StrokeKind::Fill);
    assert_eq!(classify(&rect(0.0, 0.0, 50.0, 50.0)), StrokeKind::Fill);
    // 很小的點不是線
    assert_eq!(classify(&rect(0.0, 0.0, 6.0, 6.0)), StrokeKind::Fill);
}

#[test]
fn 長條的中線在正中央且記下半寬() {
    let lines = centerlines(&rect(0.0, 0.0, 200.0, 20.0));
    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = &lines[0];
    for p in line {
        assert!((p[1] - 10.0).abs() < 2.5, "偏離中線：{p:?}");
        assert!((p[2] - 10.0).abs() < 3.0, "半寬不對：{p:?}");
    }
    let (a, b) = (line.first().unwrap(), line.last().unwrap());
    let (lo, hi) = (a[0].min(b[0]), a[0].max(b[0]));
    // 兩端要延伸到接近筆畫的盡頭
    assert!(lo < 6.0 && hi > 194.0, "兩端 {lo} {hi}");
}

#[test]
fn 轉彎的筆畫是一條連續的中線() {
    let lines = centerlines(&l_shape());
    assert_eq!(lines.len(), 1, "分成了 {} 段", lines.len());
    let line = &lines[0];
    let len: f64 = line
        .windows(2)
        .map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt())
        .sum();
    // 橫臂約 190 + 直臂約 110
    assert!((250.0..340.0).contains(&len), "長度 {len}");
}

#[test]
fn 緞面的針點左右交替且都落在筆畫範圍內() {
    let poly = rect(0.0, 0.0, 200.0, 20.0);
    let line = &centerlines(&poly)[0];
    let runs = satin(line, 1.0);
    assert_eq!(runs.len(), 1);
    let pts = &runs[0];
    assert!(pts.len() > 40, "{}", pts.len());
    let grown = poly.offset(4.0);
    for (i, p) in pts.iter().enumerate() {
        assert!(grown.contains(*p), "{p:?} 超出筆畫");
        // 左右交替：上下兩側輪流
        let upper = p[1] > 10.0;
        let first_upper = pts[0][1] > 10.0;
        assert_eq!(upper == first_upper, i % 2 == 0, "第 {i} 針沒有交替");
    }
    // 沿筆畫方向的間距約等於 SATIN_STEP，不能超過太多
    let xs: Vec<f64> = pts.iter().step_by(2).map(|p| p[0]).collect();
    for w in xs.windows(2) {
        assert!((w[1] - w[0]).abs() <= SATIN_STEP * 2.0 + 0.5, "{w:?}");
    }
    // 每針橫跨筆畫寬度，不會超過寬度加補償
    for w in pts.windows(2) {
        let d = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
        assert!(d < 20.0 + 6.0 + SATIN_STEP * 2.0, "單針太長 {d}");
    }
}

#[test]
fn 平針沿中線來回三趟且針距不超過上限() {
    let line = &centerlines(&rect(0.0, 0.0, 200.0, 6.0))[0];
    let runs = run_stitch(line, 25.0);
    assert_eq!(runs.len(), 1);
    let pts = &runs[0];
    // 去、回、再去：首尾位置依序為 左→右→左→右
    let (first, last) = (pts[0], pts[pts.len() - 1]);
    assert!(
        (first[0] < 20.0 && last[0] > 180.0) || (first[0] > 180.0 && last[0] < 20.0),
        "{first:?} {last:?}"
    );
    for w in pts.windows(2) {
        let d = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
        assert!(d <= 25.0 + 1e-6, "針距 {d}");
    }
    assert!(pts.len() >= 3 * 7, "{}", pts.len());
}

#[test]
fn 主軸方向_橫的長條是零度_直的是九十度_方塊沒有主軸() {
    let h = principal_angle(&rect(0.0, 0.0, 200.0, 40.0)).unwrap();
    assert!(h.abs() < 3.0 || (h - 180.0).abs() < 3.0, "{h}");
    let v = principal_angle(&rect(0.0, 0.0, 40.0, 200.0)).unwrap();
    assert!((v - 90.0).abs() < 3.0, "{v}");
    let d = Polygon::new(vec![vec![
        [0.0, 0.0],
        [100.0, 100.0],
        [80.0, 120.0],
        [-20.0, 20.0],
    ]]);
    let a = principal_angle(&d).unwrap();
    assert!((a - 45.0).abs() < 5.0, "{a}");
    assert!(principal_angle(&rect(0.0, 0.0, 50.0, 50.0)).is_none());
}

fn circle(cx: f64, cy: f64, r: f64, ccw: bool) -> Vec<[f64; 2]> {
    (0..72)
        .map(|i| {
            let a = i as f64 / 72.0 * std::f64::consts::TAU * if ccw { 1.0 } else { -1.0 };
            [cx + r * a.cos(), cy + r * a.sin()]
        })
        .collect()
}

#[test]
fn 圓環的中線是一圈連續的封閉線_半徑在環的正中() {
    let ring = Polygon::new(vec![
        circle(0.0, 0.0, 60.0, true),
        circle(0.0, 0.0, 40.0, false),
    ]);
    let lines = centerlines(&ring);
    assert_eq!(lines.len(), 1, "分成了 {} 段", lines.len());
    for p in &lines[0] {
        let r = (p[0] * p[0] + p[1] * p[1]).sqrt();
        assert!((r - 50.0).abs() < 4.0, "偏離環中央：半徑 {r}");
        assert!((p[2] - 10.0).abs() < 3.5, "半寬 {}", p[2]);
    }
}

#[test]
fn 十字形的中線蓋到四個端點() {
    let cross = Polygon::new(vec![vec![
        [80.0, 0.0],
        [100.0, 0.0],
        [100.0, 80.0],
        [180.0, 80.0],
        [180.0, 100.0],
        [100.0, 100.0],
        [100.0, 180.0],
        [80.0, 180.0],
        [80.0, 100.0],
        [0.0, 100.0],
        [0.0, 80.0],
        [80.0, 80.0],
    ]]);
    let lines = centerlines(&cross);
    let pts: Vec<[f64; 3]> = lines.iter().flatten().copied().collect();
    let near = |x: f64, y: f64| {
        pts.iter()
            .any(|p| (p[0] - x).abs() < 12.0 && (p[1] - y).abs() < 12.0)
    };
    assert!(near(90.0, 4.0) && near(90.0, 176.0) && near(4.0, 90.0) && near(176.0, 90.0));
}

fn star(points: usize, outer: f64, inner: f64) -> Polygon {
    Polygon::new(vec![(0..points * 2)
        .map(|i| {
            let a = i as f64 / (points * 2) as f64 * std::f64::consts::TAU;
            let r = if i % 2 == 0 { outer } else { inner };
            [r * a.cos(), r * a.sin()]
        })
        .collect()])
}

#[test]
fn 胖的星形花朵不當成筆畫_細長筆畫才是() {
    // 花朵形狀周長很長、平均寬度小，但中間是胖的：不能用緞面繡成放射狀
    assert!(plan_stroke(&star(8, 70.0, 38.0)).is_none());
    let (kind, lines) = plan_stroke(&rect(0.0, 0.0, 300.0, 20.0)).unwrap();
    assert_eq!(kind, StrokeKind::Satin);
    assert_eq!(lines.len(), 1);
    // 太短的線不是筆畫
    assert!(plan_stroke(&rect(0.0, 0.0, 30.0, 6.0)).is_none());
}

#[test]
fn 窄的緞面只走中線底針_寬的多走兩側邊線底針() {
    use stitch_gen::stroke::satin_underlay;
    let narrow = &centerlines(&rect(0.0, 0.0, 200.0, 14.0))[0]; // 1.4mm
    let wide = &centerlines(&rect(0.0, 0.0, 200.0, 30.0))[0]; // 3mm
    assert_eq!(satin_underlay(narrow).len(), 1, "窄的只要中線");
    let runs = satin_underlay(wide);
    assert_eq!(runs.len(), 3, "寬的要中線加兩側邊線");
    // 兩條邊線在筆畫內、離邊約一點點，且分居中線兩側
    let ys: Vec<f64> = runs
        .iter()
        .map(|r| r.iter().map(|p| p[1]).sum::<f64>() / r.len() as f64)
        .collect();
    let (lo, hi) = (
        ys.iter().cloned().fold(f64::MAX, f64::min),
        ys.iter().cloned().fold(f64::MIN, f64::max),
    );
    assert!(lo > 0.0 && hi < 30.0, "邊線要在筆畫內：{lo} {hi}");
    assert!(hi - lo > 15.0, "兩條邊線要分開在兩側：{lo} {hi}");
    for r in &runs {
        for w in r.windows(2) {
            let d = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
            assert!(d <= 25.0 + 1e-6, "底針針距 {d}");
        }
    }
}

#[test]
fn 緞面寬度大致一致的筆畫整條統一寬度() {
    // 寬度在 ±8% 內抖動的長條（像素階梯造成的），整條應統一成同一個寬度，邊緣才整齊
    let line: Vec<[f64; 3]> = (0..=100)
        .map(|i| {
            let jitter = [0.92, 1.0, 1.08][i % 3];
            [i as f64 * 2.0, 0.0, 10.0 * jitter]
        })
        .collect();
    let runs = satin(&line, 0.0);
    let widths: Vec<f64> = runs[0]
        .windows(2)
        .map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt())
        .collect();
    let (lo, hi) = (
        widths.iter().cloned().fold(f64::MAX, f64::min),
        widths.iter().cloned().fold(f64::MIN, f64::max),
    );
    assert!(hi - lo < 0.5, "寬度還在抖：{lo} ~ {hi}");
}

#[test]
fn 緞面寬度變化很大的筆畫維持真實寬度() {
    // 從 4 慢慢變到 16 的錐形：不能被抹平成同一個寬度
    let line: Vec<[f64; 3]> = (0..=100)
        .map(|i| [i as f64 * 2.0, 0.0, 4.0 + 12.0 * i as f64 / 100.0])
        .collect();
    let runs = satin(&line, 0.0);
    let pts = &runs[0];
    let first = (pts[1][1] - pts[0][1]).abs();
    let last = (pts[pts.len() - 1][1] - pts[pts.len() - 2][1]).abs();
    assert!(last > first * 2.0, "錐形被抹平了：{first} → {last}");
}

#[test]
fn 急轉彎的內側針不越過曲率中心_且長短交替() {
    // 半徑 12 的半圓，半寬 14（比轉彎半徑大）：內側的針若照放會越過圓心、互相交疊
    let r = 12.0;
    let line: Vec<[f64; 3]> = (0..=60)
        .map(|i| {
            let t = i as f64 / 60.0 * std::f64::consts::PI;
            [r * t.cos(), r * t.sin(), 14.0]
        })
        .collect();
    let pts = &satin(&line, 0.0)[0];
    // 內側的針不能越過圓心：越過的話，相鄰的針線會互相交叉
    let cross = |a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]| {
        let o = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
            (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
        };
        o(a, b, c) * o(a, b, d) < 0.0 && o(c, d, a) * o(c, d, b) < 0.0
    };
    let mut crossings = 0;
    for k in 0..pts.len() - 1 {
        for m in k + 2..(k + 10).min(pts.len() - 1) {
            if cross(pts[k], pts[k + 1], pts[m], pts[m + 1]) {
                crossings += 1;
            }
        }
    }
    assert_eq!(crossings, 0, "內側的針線互相交叉 {crossings} 處");
    let inner: Vec<f64> = pts
        .iter()
        .map(|p| (p[0] * p[0] + p[1] * p[1]).sqrt())
        .filter(|&d| d < r)
        .collect();
    assert!(!inner.is_empty(), "找不到內側的針");
    // 長短交替：內側針的離心距離不全相同
    let (lo, hi) = (
        inner.iter().cloned().fold(f64::MAX, f64::min),
        inner.iter().cloned().fold(f64::MIN, f64::max),
    );
    assert!(hi - lo > 1.0, "內側針沒有長短交替：{lo} ~ {hi}");
}
