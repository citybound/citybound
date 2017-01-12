#![feature(plugin)]
#![plugin(clippy)]
#![feature(conservative_impl_trait)]
#![feature(specialization)]

extern crate allocators;
mod pointer_to_maybe_compact;
#[macro_use]
mod compact;
mod compact_vec;
mod compact_dict;

pub use self::compact::Compact;
pub use self::compact_vec::CompactVec as CVec;
pub use self::compact_dict::CompactDict as CDict;