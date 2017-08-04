pub mod lane;
pub mod construction;
pub mod microtraffic;
pub mod rendering;

pub mod planning;
pub mod pathfinding;

use kay::ActorSystem;

pub fn setup(system: &mut ActorSystem) {
    self::lane::setup(system);
    self::construction::setup(system);
    self::microtraffic::setup(system);
    self::pathfinding::setup(system);
}

pub fn setup_ui(system: &mut ActorSystem) {
    rendering::setup(system);
    planning::setup(system);
}
