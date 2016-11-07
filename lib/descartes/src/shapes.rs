use super::{Shape, N, P2, THICKNESS, Norm, Curve};
use super::path::Path;
use super::primitives::Circle;

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
}

impl<P: Path> Shape for Band<P> {
    fn contains(&self, point: P2) -> bool {
        self.path.distance_to(point) < self.width + THICKNESS/2.0
    }
}