use kay::ActorSystem;

pub mod interaction;

pub fn setup(system: &mut ActorSystem) {
    interaction::setup(system);
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;