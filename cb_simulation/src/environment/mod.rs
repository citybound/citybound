use kay::ActorSystem;
pub mod vegetation;

pub fn setup(system: &mut ActorSystem) {
    vegetation::setup(system);
}
