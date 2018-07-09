use kay::{ActorSystem, World};

use simulation::SimulationID;
use planning::PlanManagerID;

pub mod resources;
pub mod market;
pub mod households;
pub mod immigration_and_development;

pub fn setup(system: &mut ActorSystem) {
    market::setup(system);
    households::setup(system);
    immigration_and_development::setup(system);
}

pub fn spawn(world: &mut World, simulation: SimulationID, plan_manager: PlanManagerID) {
    households::spawn(world);
    immigration_and_development::spawn(world, simulation, plan_manager);
}
