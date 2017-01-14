
use kay::ActorSystem;

/// Trait which must be implemented for any mod of Citybound.
pub trait CityboundMod: Sized {
    /// Called the first time the mod is loaded into a savegame.
    fn setup(&mut ActorSystem) -> Self;
}

#[doc(hidden)]
pub trait ModWrapper {
    fn setup(&mut self, &mut ActorSystem);

    fn has_instance(&mut self) -> bool;
    fn drop_instance(&mut self);
}
