use kay::ActorSystem;
pub mod lanes_and_cars;
pub mod economy;


pub fn setup(system: &mut ActorSystem) {
    lanes_and_cars::setup(system);
    economy::setup(system);
}

pub fn setup_ui(system: &mut ActorSystem) {
    lanes_and_cars::setup_ui(system);
}
