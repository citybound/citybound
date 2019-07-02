use kay::{ActorSystem, World};

pub mod buildings;
pub mod vacant_lots;
pub mod construction;
pub mod zone_planning;
pub mod ui;

pub fn setup(system: &mut ActorSystem) {
    buildings::setup(system);
    vacant_lots::setup(system);
    ui::auto_setup(system);
}

pub fn spawn(world: &mut World) {
    buildings::spawn(world);
}
