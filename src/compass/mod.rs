extern crate nalgebra;
extern crate smallvec;

use nalgebra::{Vector2, Point2, Dot, Norm};
use std::f32::consts::PI;

pub type N = f32;
pub type V2 = Vector2<N>;
pub type P2 = Point2<N>;

// Thickness radius
const THICKNESS: N = 0.0001;
const ROUGH_TOLERANCE: N = 0.0000001;

mod primitives;
mod path;
mod intersect;

pub use self::primitives::*;
pub use self::path::{Path};

fn angle_to(a: V2, b: V2) -> N {
    let theta: N = a.dot(&b) / (a.norm() * b.norm());
    return theta.min(1.0).max(-1.0).acos();
}

fn angle_along_to(a: V2, a_direction: V2, b: V2) -> N {
    let simple_angle = angle_to(a, b);
    let linear_direction = (b - a).normalize();

    if a_direction.dot(&linear_direction) >= 0.0 {
        return simple_angle;
    } else {
        return 2.0 * PI - simple_angle;
    };
}

pub trait WithUniqueOrthogonal {
    fn orthogonal(&self) -> Self;
}

impl WithUniqueOrthogonal for V2 {
    fn orthogonal(&self) -> V2 {
        V2::new(self.y, -self.x)
    }
}

trait RoughlyComparable {
    fn is_roughly(&self, other: Self) -> bool;
}

impl RoughlyComparable for N {
    fn is_roughly(&self, other: N) -> bool {
        return (self - other).abs() <= ROUGH_TOLERANCE;
    }
}

pub trait Curve {
    fn project(&self, point: P2) -> Option<N>;
    fn includes(&self, point: P2) -> bool;
    fn distance_to(&self, point: P2) -> N;
}

pub trait FiniteCurve : Curve {
    fn length(&self) -> N;
    fn along(&self, distance: N) -> P2;
    fn direction_along(&self, distance: N) -> V2;
    fn start_direction(&self) -> V2 {
        self.direction_along(0.0)
    }
    fn end_direction(&self) -> V2 {
        self.direction_along(self.length())
    }
    fn subsection(&self, start: N, end: N) -> Self;
}