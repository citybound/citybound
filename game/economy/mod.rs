use kay::ActorSystem;

use simulation::SimulationID;
use planning::PlanManagerID;

pub mod resources;
pub mod market;
pub mod households;
pub mod immigration_and_development;

pub fn setup(system: &mut ActorSystem, simulation: SimulationID, plan_manager: PlanManagerID) {
    market::setup(system);
    households::setup(system);
    immigration_and_development::setup(system, simulation, plan_manager);
}
