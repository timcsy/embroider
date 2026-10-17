use crate::Ring;
use kurbo::{BezPath, PathEl};

/// 把貝茲路徑展平成折線環。`tolerance` 是允許的最大偏差。
/// 每個子路徑一個環；環隱含閉合，不重複起點。
pub fn flatten_path(path: &BezPath, tolerance: f64) -> Vec<Ring> {
    let mut rings: Vec<Ring> = Vec::new();
    let mut current: Ring = Vec::new();
    kurbo::flatten(path.elements().iter().copied(), tolerance, |el| match el {
        PathEl::MoveTo(p) => {
            if current.len() > 1 {
                rings.push(std::mem::take(&mut current));
            }
            current.clear();
            current.push([p.x, p.y]);
        }
        PathEl::LineTo(p) => current.push([p.x, p.y]),
        PathEl::ClosePath => {
            if current.len() > 1 {
                rings.push(std::mem::take(&mut current));
            }
            current.clear();
        }
        _ => {}
    });
    if current.len() > 1 {
        rings.push(current);
    }
    rings
}
