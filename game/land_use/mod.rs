use kay::ActorSystem;
use core::simulation::SimulationID;
use stagemaster::UserInterfaceID;

pub mod buildings;
pub mod zone_planning_new;

pub fn setup(system: &mut ActorSystem, user_interface: UserInterfaceID, simulation: SimulationID) {
    buildings::setup(system, user_interface, simulation);
}