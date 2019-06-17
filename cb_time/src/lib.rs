// TODO: remove once https://github.com/rust-lang/rust/issues/54726 is resolved
#![feature(custom_inner_attributes)]
#![allow(clippy::new_without_default)]
extern crate kay;
extern crate compact;
#[macro_use]
extern crate compact_macros;
#[macro_use]
extern crate serde_derive;

pub mod units;
pub mod actors;
