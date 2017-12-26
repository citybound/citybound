use kay::ActorSystem;

pub mod resources;
pub mod market;
pub mod households;

pub fn setup(system: &mut ActorSystem) {
    market::setup(system);
    households::setup(system);
}
