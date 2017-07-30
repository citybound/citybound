use super::{Shape, N, P2, THICKNESS, Norm, Curve, FiniteCurve};
use super::path::Path;
use super::primitives::{Circle, Segment};

impl Shape for Circle {
    fn contains(&self, point: P2) -> bool {
        (point - self.center).norm() <= self.radius + THICKNESS / 2.0
    }
}

#[derive(Clone)]
pub struct Band<P: Path> {
    pub path: P,
    pub width: N,
}

impl<P: Path> Band<P> {
    pub fn new(path: P, width: N) -> Band<P> {
        Band { path: path, width: width }
    }

    pub fn outline(&self) -> P {
        if let (Some(left_path), Some(right_path)) =
            (
                self.path.shift_orthogonally(-self.width / 2.0),
                self.path.shift_orthogonally(self.width / 2.0).map(|p| {
                    p.reverse()
                }),
            )
        {
            let connector1 = Segment::line(left_path.end(), right_path.start());
            let connector2 = Segment::line(right_path.end(), left_path.start());
            P::new(
                left_path
                    .segments()
                    .iter()
                    .chain(&[connector1])
                    .chain(right_path.segments().iter())
                    .chain(&[connector2])
                    .cloned()
                    .collect(),
            )
        } else {
            self.path.clone()
        }
    }

    pub fn outline_distance_to_path_distance(&self, distance: N) -> N {
        if let (Some(left_path_length), Some(right_path_length)) =
            (
                self.path.shift_orthogonally(-self.width / 2.0).map(|p| {
                    p.length()
                }),
                self.path.shift_orthogonally(self.width / 2.0).map(
                    |p| p.length(),
                ),
            )
        {
            if distance > left_path_length + self.width + right_path_length {
                // on connector2
                0.0
            } else if distance > left_path_length + self.width {
                // on right side
                (1.0 - (distance - left_path_length - self.width) / right_path_length) *
                    self.path.length()
            } else if distance > left_path_length {
                // on connector1
                self.path.length()
            } else {
                // on left side
                (distance / left_path_length) * self.path.length()
            }
        } else {
            distance
        }
    }
}

impl<P: Path> Shape for Band<P> {
    fn contains(&self, point: P2) -> bool {
        if let Some(along) = self.path.project(point) {
            let distance = (point - self.path.along(along)).norm();
            distance < self.width / 2.0 + THICKNESS / 2.0
        } else {
            false
        }
    }
}
