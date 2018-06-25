use {N, P2, V2, NEG_INFINITY, INFINITY};

#[derive(Copy, Clone)]
pub struct BoundingBox {
    pub min: P2,
    pub max: P2,
}

impl BoundingBox {
    pub fn infinite() -> Self {
        BoundingBox {
            min: P2::new(NEG_INFINITY, NEG_INFINITY),
            max: P2::new(INFINITY, INFINITY),
        }
    }

    pub fn overlaps(&self, other: &BoundingBox) -> bool {
        self.max.x >= other.min.x
            && other.max.x >= self.min.x
            && self.max.y >= other.min.y
            && other.max.y >= self.min.y
    }

    pub fn point(p: P2) -> Self {
        BoundingBox { min: p, max: p }
    }

    pub fn grown_by(&self, offset: N) -> Self {
        BoundingBox {
            min: self.min - V2::new(offset, offset),
            max: self.max + V2::new(offset, offset),
        }
    }
}

pub trait HasBoundingBox {
    fn bounding_box(&self) -> BoundingBox;
}
