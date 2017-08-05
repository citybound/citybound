
use std::io;
use std::error::Error;
use std::path::Path;
use std::str::FromStr;

use kay::ActorSystem;

use packages::{PackageManager, Ident};
use modules::ModuleManager;

/// The weaver takes care of loading and keeping track of mods.
#[derive(Default)]
pub struct Weaver {
    packages: PackageManager,
    modules: ModuleManager,
}

impl Weaver {
    pub fn new() -> Weaver {
        Weaver::default()
    }

    /// The system mods are a bit special, they are mods shipped
    /// with the game and usually enables some basic functionality.
    ///
    /// Mods in this folder can be replaced by passing a source
    /// folder to weaver.
    pub fn add_system_mods<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        self.packages.add_packages(path, &["system"], false)
    }

    /// Folder containing the source of system mods.
    /// Useful for development.
    pub fn add_source_mods<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        self.packages.add_packages(path, &["system"], true)
    }

    /// Adds a folder containing normal mods.
    pub fn add_mods<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        unimplemented!();
    }

    /// Drops all currently loaded modules.
    pub fn drop_modules(&mut self) {
        self.modules.drop_modules();
    }

    /// Loads a package into the game.
    ///
    /// `name` can either be an ident or just the name of the mod,
    /// in case of the latter, the latest version will be chosen.
    pub fn load_package(&mut self, name: &str, system: &mut ActorSystem) -> Result<(), Box<Error>> {
        let path = PackagePath::from_str(name)?;
        let latest = self.packages
            .resolve(&path)
            .latest()
            .ok_or_else(|| format!("couldn't resolve: {:?}", name))?;

        self.modules
            .load(latest.ident(), &self.packages, system)?;
        Ok(())
    }
}
