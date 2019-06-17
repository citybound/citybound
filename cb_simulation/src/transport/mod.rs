pub mod lane;
pub mod construction;
pub mod microtraffic;
pub mod ui;

pub mod transport_planning;
pub mod pathfinding;

use kay::{ActorSystem, World};
use cb_time::actors::TimeID;

pub fn setup(system: &mut ActorSystem) {
    self::lane::setup(system);
    self::construction::setup(system);
    self::microtraffic::setup(system);
    self::pathfinding::setup(system);
    self::ui::setup(system);
}

pub fn spawn(world: &mut World, time: TimeID) {
    self::pathfinding::spawn(world, time);
}
