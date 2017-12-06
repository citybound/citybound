use kay::ActorSystem;

pub mod resources;
pub mod market;
pub mod households;
pub mod buildings;

use stagemaster::UserInterfaceID;
use core::simulation::SimulationID;
use planning::materialized_reality::MaterializedRealityID;

pub fn setup(
    system: &mut ActorSystem,
    user_interface: UserInterfaceID,
    simulation: SimulationID,
    materialized_reality: MaterializedRealityID,
) {
    market::setup(system);
    households::setup(system);
    buildings::setup(system, user_interface, simulation, materialized_reality);
}
