#![feature(plugin)]
#![plugin(clippy)]
extern crate nalgebra;
extern crate ncollide_transformation;
extern crate ordered_float;

use nalgebra::{Vector2, Point2, Vector3, Vector4, Point3, Isometry3, Perspective3, Matrix4};
pub use nalgebra::{Dot, ToHomogeneous, Norm, Inverse, Rotate};
use std::f32::consts::PI;

pub type N = f32;
pub type V2 = Vector2<N>;
pub type P2 = Point2<N>;
pub type V3 = Vector3<N>;
pub type V4 = Vector4<N>;
pub type P3 = Point3<N>;
pub type M4 = Matrix4<N>;
pub type Iso3 = Isometry3<N>;
pub type Persp3 = Perspective3<N>;

// Thickness radius
const THICKNESS: N = 0.001;
const ROUGH_TOLERANCE: N = 0.0000001;

mod primitives;
mod path;
mod intersect;
mod shapes;

pub use self::primitives::*;
pub use self::path::{Path, convex_hull};
pub use self::intersect::*;
pub use self::shapes::*;

fn angle_to(a: V2, b: V2) -> N {
    let theta: N = a.dot(&b) / (a.norm() * b.norm());
    theta.min(1.0).max(-1.0).acos()
}

fn angle_along_to(a: V2, a_direction: V2, b: V2) -> N {
    let simple_angle = angle_to(a, b);
    let linear_direction = (b - a).normalize();

    if a_direction.dot(&linear_direction) >= 0.0 {
        simple_angle
    } else {
        2.0 * PI - simple_angle
    }
}

pub trait WithUniqueOrthogonal {
    fn orthogonal(&self) -> Self;
}

impl WithUniqueOrthogonal for V2 {
    fn orthogonal(&self) -> V2 {
        V2::new(self.y, -self.x)
    }
}

pub trait RelativeToBasis {
    fn to_basis(self, basis_x: Self) -> Self;
    fn from_basis(self, basis_x: Self) -> Self;
}

impl RelativeToBasis for V2 {
    fn to_basis(self, basis_x: V2) -> V2 {
        V2::new(
            basis_x.dot(&self),
            basis_x.orthogonal().dot(&self)
        )
    }

    fn from_basis(self, basis_x: V2) -> V2 {
        self.x * basis_x +self.y * basis_x.orthogonal()
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

pub trait RoughlyComparable : Sized {
    fn is_roughly(&self, other: Self) -> bool {
        self.is_roughly_within(other, ROUGH_TOLERANCE)
    }
    fn is_roughly_within(&self, other: Self, tolerance: N) -> bool;
}

impl RoughlyComparable for N {
    fn is_roughly_within(&self, other: N, tolerance: N) -> bool {
        (self - other).abs() <= tolerance
    }
}

impl RoughlyComparable for P2 {
    fn is_roughly_within(&self, other: P2, tolerance: N) -> bool {
        (*self - other).norm() <= tolerance
    }
}

impl RoughlyComparable for V2 {
    fn is_roughly_within(&self, other: V2, tolerance: N) -> bool {
        (*self - other).norm() <= tolerance
    }
}

pub trait Curve : Sized {
    fn project_with_max_distance(&self, point: P2, max_distance: N) -> Option<N> {
        self.project(point).and_then(|offset|
            if self.distance_to(point) < max_distance {Some(offset)} else {None}
        )
    }
    fn project(&self, point: P2) -> Option<N>;
    fn includes(&self, point: P2) -> bool {
        self.distance_to(point) < THICKNESS/2.0
    }
    fn distance_to(&self, point: P2) -> N;
}

pub trait FiniteCurve : Curve {
    fn length(&self) -> N;
    fn along(&self, distance: N) -> P2;
    fn direction_along(&self, distance: N) -> V2;
    fn start(&self) -> P2;
    fn start_direction(&self) -> V2 {
        self.direction_along(0.0)
    }
    fn end(&self) -> P2;
    fn end_direction(&self) -> V2 {
        self.direction_along(self.length())
    }
    fn reverse(&self) -> Self;
    fn subsection(&self, start: N, end: N) -> Option<Self>;
    fn shift_orthogonally(&self, shift_to_right: N) -> Option<Self>;
}

pub trait Shape {
    fn contains(&self, point: P2) -> bool;
}