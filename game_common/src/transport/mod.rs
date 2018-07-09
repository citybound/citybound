pub mod lane;
pub mod construction;
pub mod microtraffic;
pub mod rendering;

pub mod transport_planning;
pub mod pathfinding;

use kay::{ActorSystem, World};
use simulation::SimulationID;

pub fn setup(system: &mut ActorSystem) {
    self::lane::setup(system);
    self::construction::setup(system);
    self::microtraffic::setup(system);
    self::pathfinding::setup(system);
    self::rendering::setup(system);
    self::transport_planning::setup(system);
}

pub fn spawn(world: &mut World, simulation: SimulationID) {
    self::pathfinding::spawn(world, simulation);
    self::rendering::spawn(world);
}
