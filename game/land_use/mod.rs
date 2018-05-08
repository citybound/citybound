use kay::ActorSystem;
use stagemaster::UserInterfaceID;

pub mod buildings;
pub mod vacant_lots;
pub mod construction;
pub mod zone_planning;

pub fn setup(system: &mut ActorSystem, user_interface: UserInterfaceID) {
    buildings::setup(system, user_interface);
    vacant_lots::setup(system);
}
