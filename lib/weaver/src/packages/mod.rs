//! Module containing helpers for loading, defining and working
//! with mods.

pub mod desc;
mod manager;
mod ident;

pub use self::desc::{Package, Info, Dependency};
pub use self::manager::{PackageManager, ResolvedPackage};
pub use self::ident::{Ident, UniqueIdent};
