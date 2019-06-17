use kay::{ActorSystem, World};

use cb_time::actors::TimeID;
use planning::CBPlanManagerID;

pub mod resources;
pub mod market;
pub mod households;
pub mod immigration_and_development;

pub fn setup(system: &mut ActorSystem) {
    market::setup(system);
    households::setup(system);
    immigration_and_development::setup(system);
}

pub fn spawn(world: &mut World, time: TimeID, plan_manager: CBPlanManagerID) {
    market::spawn(world);
    households::spawn(world);
    immigration_and_development::spawn(world, time, plan_manager);
}
