use kay::ActorSystem;

pub mod resources;
pub mod market;
pub mod households;
pub mod buildings;

pub fn setup(system: &mut ActorSystem) {
    resources::setup();
    market::setup(system);
    households::setup(system);
    buildings::setup(system);
}

pub fn setup_ui(system: &mut ActorSystem) {
    buildings::setup_ui(system);
}
