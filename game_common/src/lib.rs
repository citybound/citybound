#![feature(custom_derive, iter_rfold)]
#![allow(dead_code)]
#![cfg_attr(feature = "cargo-clippy", allow(trivially_copy_pass_by_ref))]
#![recursion_limit = "128"]
// Enable this for memory tracking with Instruments/MacOS
// and for much better stacktraces for memory issues
#![cfg_attr(feature = "server", feature(alloc_system))]
#[cfg(feature = "server")]
extern crate alloc_system;

extern crate ordered_float;
extern crate itertools;
extern crate rand;
extern crate fnv;
extern crate roaring;
extern crate backtrace;
extern crate uuid;

pub extern crate compact;
#[macro_use]
extern crate compact_macros;
pub extern crate kay;
pub extern crate michelangelo;
pub extern crate descartes;

#[macro_use]
extern crate serde_derive;

pub const ENV_NAME: &str = "Citybound";
pub const ENV_AUTHOR: &str = "ae play";
pub const ENV_VERSION: &str = "0.4.0";

pub mod util;
pub mod simulation;
pub mod transport;
pub mod planning;
pub mod construction;
pub mod economy;
pub mod land_use;
pub mod style;

pub fn setup_common(system: &mut kay::ActorSystem) {
    for setup_fn in &[
        simulation::setup,
        planning::setup,
        construction::setup,
        transport::setup,
        economy::setup,
        land_use::setup,
    ] {
        setup_fn(system)
    }
}
