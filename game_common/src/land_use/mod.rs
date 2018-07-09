use kay::{ActorSystem, World};
use stagemaster::UserInterfaceID;

pub mod buildings;
pub mod vacant_lots;
pub mod construction;
pub mod zone_planning;

pub fn setup(system: &mut ActorSystem) {
    buildings::setup(system);
    vacant_lots::setup(system);
}

pub fn spawn(world: &mut World, user_interface: UserInterfaceID) {
    buildings::spawn(world, user_interface)
}
