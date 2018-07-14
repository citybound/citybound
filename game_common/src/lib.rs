#![feature(custom_derive, iter_rfold)]
#![allow(dead_code)]
// Enable this for memory tracking with Instruments/MacOS
// and for much better stacktraces for memory issues
#![feature(alloc_system)]
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
#[cfg(feature = "server")]
pub extern crate monet;
#[cfg(feature = "browser")]
pub extern crate browser_monet;
#[cfg(feature = "browser")]
pub use browser_monet as monet;
pub extern crate descartes;
#[cfg(feature = "server")]
pub extern crate stagemaster;
#[cfg(feature = "browser")]
pub extern crate browser_stagemaster;
#[cfg(feature = "browser")]
pub use browser_stagemaster as stagemaster;
#[cfg(feature = "server")]
#[macro_use]
extern crate imgui;
#[macro_use]
extern crate serde_derive;
#[cfg(feature = "browser")]
#[macro_use]
extern crate stdweb;

use stagemaster::environment::Environment;

pub const ENV: &Environment = &Environment {
    name: "Citybound",
    author: "ae play",
    version: "0.4.0",
};

pub mod util;
pub mod simulation;
pub mod transport;
pub mod planning;
pub mod construction;
pub mod economy;
pub mod land_use;
pub mod ui_layers;
pub mod render_layers;
pub mod style;
pub mod browser_ui;

pub fn setup_all(system: &mut kay::ActorSystem) {
    for setup_fn in &[
        stagemaster::setup,
        simulation::setup,
        ui_layers::setup,
        planning::setup,
        construction::setup,
        transport::setup,
        economy::setup,
        land_use::setup,
        browser_ui::setup,
    ] {
        setup_fn(system)
    }
}
