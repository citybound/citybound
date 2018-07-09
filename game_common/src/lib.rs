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
pub extern crate monet;
pub extern crate descartes;
pub extern crate stagemaster;
#[macro_use]
extern crate imgui;
#[macro_use]
extern crate serde_derive;
extern crate tungstenite;
extern crate rmpv;

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

use kay::Actor;
use compact::CVec;
use monet::Grouper;
use transport::lane::{Lane, SwitchLane};
use transport::rendering::LaneRenderer;
use economy::households::family::Family;
use economy::households::grocery_shop::GroceryShop;
use economy::households::grain_farm::GrainFarm;
use economy::households::cow_farm::CowFarm;
use economy::households::mill::Mill;
use economy::households::bakery::Bakery;
use economy::households::neighboring_town_trade::NeighboringTownTrade;
use economy::households::tasks::TaskEndScheduler;
use land_use::buildings::rendering::BuildingRenderer;
use planning::PlanManager;
use construction::Construction;

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
