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

use nalgebra::{Vector2, Point2, Rotation2,
Vector3, Point3, Isometry3, Affine3, Perspective3,
Vector4, Matrix4, dot};
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

mod rough_eq;
mod angles;
mod convert_2d_3d;
mod bbox;
mod line_path;
mod closed_line_path;
mod curved_path;
mod intersect;
mod areas;
mod band;

pub use self::rough_eq::*;
pub use self::angles::*;
pub use self::convert_2d_3d::*;
pub use self::bbox::*;
pub use self::line_path::*;
pub use self::closed_line_path::*;
pub use self::curved_path::*;
pub use self::intersect::*;
pub use self::areas::*;
pub use self::band::*;
