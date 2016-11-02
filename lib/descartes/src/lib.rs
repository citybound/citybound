extern crate nalgebra;
extern crate smallvec;

use nalgebra::{Vector2, Point2, Vector3, Point3, Isometry3, Perspective3};
pub use nalgebra::{Dot, ToHomogeneous, Norm};
use std::f32::consts::PI;

pub type N = f32;
pub type V2 = Vector2<N>;
pub type P2 = Point2<N>;
pub type V3 = Vector3<N>;
pub type P3 = Point3<N>;
pub type Iso3 = Isometry3<N>;
pub type Persp3 = Perspective3<N>;

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

pub trait Into2d {
    type Target;
    fn into_2d(self) -> Self::Target;
}

impl Into2d for V3 {
    type Target = V2;
    fn into_2d(self) -> V2 {
        V2::new(self.x, self.y)
    }
}

impl Into2d for P3 {
    type Target = P2;
    fn into_2d(self) -> P2 {
        P2::new(self.x, self.y)
    }
}

pub trait Into3d {
    type Target;
    fn into_3d(self) -> Self::Target;
}

impl Into3d for V2 {
    type Target = V3;
    fn into_3d(self) -> V3 {
        V3::new(self.x, self.y, 0.0)
    }
}

impl Into3d for P2 {
    type Target = P3;
    fn into_3d(self) -> P3 {
        P3::new(self.x, self.y, 0.0)
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