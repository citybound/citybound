use kay::ActorSystem;

pub mod resources;
pub mod market;
pub mod households;
pub mod buildings;

use stagemaster::UserInterfaceID;
use core::simulation::SimulationID;

pub fn setup(system: &mut ActorSystem, user_interface: UserInterfaceID, simulation: SimulationID) {
    resources::setup();
    market::setup(system);
    households::setup(system);
    buildings::setup(system, user_interface, simulation);
}
