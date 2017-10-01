pub mod lane;
pub mod construction;
pub mod microtraffic;
pub mod rendering;

pub mod planning;
pub mod pathfinding;

use kay::ActorSystem;
use stagemaster::UserInterfaceID;
use monet::RendererID;
use core::simulation::SimulationID;

pub fn setup(
    system: &mut ActorSystem,
    user_interface: UserInterfaceID,
    renderer_id: RendererID,
    simulation: SimulationID,
) {
    self::lane::setup(system);
    let materialized_reality = self::construction::setup(system);
    self::microtraffic::setup(system);
    self::pathfinding::setup(system, simulation);
    self::rendering::setup(system);
    self::planning::setup(system, user_interface, renderer_id, materialized_reality);
}
