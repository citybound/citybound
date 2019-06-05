#![recursion_limit = "128"]
// TODO: remove once https://github.com/rust-lang/rust/issues/54726 is resolved
#![feature(custom_inner_attributes)]
extern crate ordered_float;
extern crate itertools;
extern crate rand;
extern crate noise;
extern crate fnv;
extern crate roaring;
extern crate uuid;
extern crate arrayvec;
extern crate cb_util;
extern crate cb_time;

pub extern crate compact;
#[macro_use]
extern crate compact_macros;
pub extern crate kay;
pub extern crate michelangelo;
pub extern crate descartes;

#[macro_use]
extern crate serde_derive;

pub mod transport;
pub mod planning;
pub mod economy;
pub mod land_use;
pub mod dimensions;
pub mod environment;

pub fn setup_common(system: &mut kay::ActorSystem) {
    for setup_fn in &[
        time::setup,
        log::setup,
        planning::setup,
        construction::setup,
        transport::setup,
        economy::setup,
        land_use::setup,
        environment::setup,
    ] {
        setup_fn(system)
    }
}
