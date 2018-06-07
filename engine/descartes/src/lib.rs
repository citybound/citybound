extern crate nalgebra;
extern crate ordered_float;
extern crate itertools;

#[cfg(feature = "compact_containers")]
extern crate compact;

#[cfg(feature = "compact_containers")]
#[macro_use]
extern crate compact_macros;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

use nalgebra::{Vector2, Point2, Vector3, Vector4, Point3, Isometry3, Affine3, Perspective3,
Matrix4, dot};
pub use nalgebra::try_inverse;

#[cfg(feature = "compact_containers")]
pub type VecLike<T> = compact::CVec<T>;

#[cfg(not(feature = "compact_containers"))]
pub type VecLike<T> = Vec<T>;

pub type N = f32;
use std::f32::consts::PI;
use std::f32::{INFINITY, NEG_INFINITY};

pub type V2 = Vector2<N>;
pub type P2 = Point2<N>;
pub type V3 = Vector3<N>;
pub type V4 = Vector4<N>;
pub type P3 = Point3<N>;
pub type M4 = Matrix4<N>;
pub type Iso3 = Isometry3<N>;
pub type Aff3 = Affine3<N>;
pub type Persp3 = Perspective3<N>;

// Thickness radius
const THICKNESS: N = 0.001;
const ROUGH_TOLERANCE: N = 0.000_000_1;

mod curves;
mod path;
mod intersect;
mod areas;

pub use self::curves::*;
pub use self::path::Path;
pub use self::intersect::*;
pub use self::areas::*;

pub fn angle_to(a: V2, b: V2) -> N {
    let theta: N = dot(&a, &b) / (a.norm() * b.norm());
    theta.min(1.0).max(-1.0).acos()
}

pub fn angle_along_to(a: V2, a_direction: V2, b: V2) -> N {
    let simple_angle = angle_to(a, b);
    let linear_direction = (b - a).normalize();

    if a_direction.dot(&linear_direction) >= 0.0 {
        simple_angle
    } else {
        2.0 * PI - simple_angle
    }
}

pub fn signed_angle_to(a: V2, b: V2) -> N {
    // https://stackoverflow.com/a/2150475
    let det = a.x * b.y - a.y * b.x;
    let dot = a.x * b.x + a.y * b.y;
    (det).atan2(dot)
}

//
//  DESCARTES ASSUMES
//  A RIGHT HAND COORDINATE SYSTEM
//

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
        V2::new(basis_x.dot(&self), basis_x.orthogonal().dot(&self))
    }

    fn from_basis(self, basis_x: V2) -> V2 {
        self.x * basis_x + self.y * basis_x.orthogonal()
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

pub trait RoughEq: Sized {
    fn rough_eq(&self, other: Self) -> bool {
        self.rough_eq_by(other, ROUGH_TOLERANCE)
    }
    fn rough_eq_by(&self, other: Self, tolerance: N) -> bool;
}

impl RoughEq for N {
    fn rough_eq_by(&self, other: N, tolerance: N) -> bool {
        (self - other).abs() <= tolerance
    }
}

impl RoughEq for P2 {
    fn rough_eq_by(&self, other: P2, tolerance: N) -> bool {
        (*self - other).norm() <= tolerance
    }
}

impl RoughEq for V2 {
    fn rough_eq_by(&self, other: V2, tolerance: N) -> bool {
        (*self - other).norm() <= tolerance
    }
}

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
