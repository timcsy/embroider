use i_overlay::core::fill_rule::FillRule;
use i_overlay::float::simplify::SimplifyShape;

pub type Pt = [f64; 2];
pub type Ring = Vec<Pt>;

/// 由多個環組成的多邊形，以「非零環繞」規則填充：
/// 同向的環重疊會合併，反向的環形成洞。
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct Polygon {
    pub rings: Vec<Ring>,
}

pub(crate) fn signed_area(ring: &[Pt]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

impl Polygon {
    pub fn new(rings: Vec<Ring>) -> Self {
        Self { rings }
    }

    pub fn is_empty(&self) -> bool {
        self.rings.is_empty()
    }

    pub fn bounds(&self) -> Option<(Pt, Pt)> {
        let mut it = self.rings.iter().flatten();
        let first = *it.next()?;
        let (mut min, mut max) = (first, first);
        for p in it {
            min = [min[0].min(p[0]), min[1].min(p[1])];
            max = [max[0].max(p[0]), max[1].max(p[1])];
        }
        Some((min, max))
    }

    /// 整理成外框逆時針、洞順時針、無重疊的形式。
    pub fn normalized(&self) -> Polygon {
        Polygon {
            rings: self.shapes().into_iter().flat_map(|s| s.rings).collect(),
        }
    }

    /// 拆成互不相連的形狀，每個形狀含一個外框與它自己的洞。
    pub fn shapes(&self) -> Vec<Polygon> {
        if self.rings.is_empty() {
            return Vec::new();
        }
        self.rings
            .simplify_shape(FillRule::NonZero)
            .into_iter()
            .map(|contours| Polygon { rings: contours })
            .collect()
    }

    /// 實際被填滿的面積。
    pub fn area(&self) -> f64 {
        self.normalized().rings.iter().map(|r| signed_area(r)).sum()
    }

    pub fn contains(&self, p: Pt) -> bool {
        crate::scanline::spans_at(self, p[1])
            .iter()
            .any(|&(a, b)| a <= p[0] && p[0] <= b)
    }

    pub fn translate(&self, dx: f64, dy: f64) -> Polygon {
        Polygon {
            rings: self
                .rings
                .iter()
                .map(|r| r.iter().map(|p| [p[0] + dx, p[1] + dy]).collect())
                .collect(),
        }
    }

    pub fn scale(&self, s: f64) -> Polygon {
        Polygon {
            rings: self
                .rings
                .iter()
                .map(|r| r.iter().map(|p| [p[0] * s, p[1] * s]).collect())
                .collect(),
        }
    }
}
