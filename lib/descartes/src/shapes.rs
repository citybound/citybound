use super::{Shape, N, P2, THICKNESS, Norm, Curve, FiniteCurve};
use super::path::Path;
use super::primitives::{Circle, Segment};

impl Shape for Circle {
    fn contains(&self, point: P2) -> bool {
        (point - self.center).norm() <= self.radius + THICKNESS/2.0
    }
}

#[derive(Clone)]
pub struct Band<P: Path> {
    pub path: P,
    pub width: N
}

impl<P: Path> Band<P> {
    pub fn new(path: P, width: N) -> Band<P> {
        Band{path: path, width: width}
    }

    pub fn outline(&self) -> P {
        let left_path = self.path.shift_orthogonally(-self.width/2.0);
        let right_path = self.path.shift_orthogonally(self.width/2.0).reverse();
        let connector1 = Segment::line(left_path.end(), right_path.start());
        let connector2 = Segment::line(right_path.end(), left_path.start());
        P::new(left_path.segments().iter()
            .chain(&[connector1])
            .chain(right_path.segments().iter())
            .chain(&[connector2])
            .cloned()
            .collect()
        )
    }
}

impl<P: Path> Shape for Band<P> {
    fn contains(&self, point: P2) -> bool {
        self.path.distance_to(point) < self.width + THICKNESS/2.0
    }
}