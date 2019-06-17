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
extern crate cb_planning;

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
        cb_time::actors::setup,
        cb_util::log::setup,
        cb_planning::plan_manager::setup::<planning::CBPlanningLogic>,
        cb_planning::construction::setup::<planning::CBPrototypeKind>,
        transport::setup,
        economy::setup,
        land_use::setup,
        environment::setup,
    ] {
        setup_fn(system)
    }
}

pub fn spawn_for_server(world: &mut kay::World) -> cb_time::actors::TimeID {
    cb_util::log::spawn(world);
    let time = cb_time::actors::spawn(world);
    let plan_manager = cb_planning::plan_manager::spawn::<planning::CBPlanningLogic>(world);
    cb_planning::construction::spawn::<planning::CBPrototypeKind>(world);
    transport::spawn(world, time);
    economy::spawn(world, time, plan_manager);
    environment::vegetation::spawn(world, plan_manager);
    time
}
