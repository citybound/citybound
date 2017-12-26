use kay::ActorSystem;
use core::simulation::SimulationID;
use stagemaster::UserInterfaceID;
use planning::materialized_reality::MaterializedRealityID;

pub mod buildings;

pub fn setup(
    system: &mut ActorSystem,
    user_interface: UserInterfaceID,
    simulation: SimulationID,
    materialized_reality: MaterializedRealityID,
) {
    buildings::setup(system, user_interface, simulation, materialized_reality);
}