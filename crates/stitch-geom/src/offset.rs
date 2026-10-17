use crate::Polygon;
use i_overlay::mesh::float::outline::offset::OutlineOffset;
use i_overlay::mesh::float::style::OutlineStyle;

impl Polygon {
    /// 偏移：正值向外擴張，負值向內收縮。收縮到消失時回傳空多邊形。
    pub fn offset(&self, distance: f64) -> Polygon {
        let shapes: Vec<Vec<Vec<[f64; 2]>>> = self.shapes().into_iter().map(|s| s.rings).collect();
        if shapes.is_empty() {
            return Polygon::default();
        }
        let out = shapes.outline(&OutlineStyle::new(distance));
        Polygon {
            rings: out.into_iter().flatten().collect(),
        }
    }
}
