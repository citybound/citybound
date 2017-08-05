
use std::error::Error;

use kay::ActorSystem;

use packages::Package;

/// Trait which must be implemented for any mod of Citybound.
pub trait CityboundMod: Sized {
    /// Called the first time the mod is loaded into a savegame.
    fn setup(&mut ActorSystem) -> Self;

    /// Called before a dependant of this package is loaded.
    ///
    /// This function should fast, because it cannot run in parallel.
    /// Please use `dep_res` for loading non-critical things.
    fn dep_setup(&mut self, &Package, &mut ActorSystem) -> Option<Module> {
        None
    }

    /// Used to do heavy loading of packages. Might be run in parallel.
    fn dep_res(&mut self, &Package) {}
}
