
use kay::ActorSystem;
use super::LoadingPackage;

/// Trait which must be implemented for any mod of Citybound.
pub trait CityboundMod: Sized {
    /// Called the first time the mod is loaded into a savegame.
    fn setup(&mut ActorSystem) -> Self;

    /// Called before a dependant of this package is loaded.
    fn dependant_loading(&mut self, &mut LoadingPackage, &mut ActorSystem) -> Result<(), String> {
        Ok(())
    }
}

#[doc(hidden)]
pub trait ModWrapper {
    fn setup(&mut self, &mut ActorSystem);
    fn dependant_loading(&mut self, &mut LoadingPackage, &mut ActorSystem) -> Result<(), String>;

    fn has_instance(&mut self) -> bool;
    fn drop_instance(&mut self);
}
