use crate::Run;
use stitch_geom::Polygon;

/// 沿多邊形每個環走一圈（含洞），針距不超過 `spacing`；每圈首尾相接。
pub fn outline(poly: &Polygon, spacing: f64) -> Vec<Run> {
    poly.normalized()
        .rings
        .iter()
        .filter(|r| r.len() >= 3)
        .map(|ring| {
            let mut run: Run = Vec::new();
            for i in 0..ring.len() {
                let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
                let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
                let n = (len / spacing).ceil().max(1.0) as usize;
                for j in 0..n {
                    let t = j as f64 / n as f64;
                    run.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
                }
            }
            run.push(run[0]);
            run
        })
        .collect()
}
