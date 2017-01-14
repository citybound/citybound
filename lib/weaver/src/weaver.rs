
use std::mem;
use std::collections::HashMap;
use std::io::{self, Read};
use std::fs::File;
use std::path::{PathBuf, Path};

use libloading::{Library, Symbol};
use kay::ActorSystem;
use semver::Version;
use serde::Deserialize;
use toml;

use ::{Package, ModWrapper};

struct LocatedPackage {
    package: Package,
    path: PathBuf,
}

/// The weaver takes care of loading and keeping track of mods.
#[derive(Default)]
pub struct Weaver {
    packages: HashMap<String, LocatedPackage>,
    loaded: HashMap<String, LoadedMod>,
}

impl Weaver {
    pub fn new() -> Weaver {
        Weaver::default()
    }

    pub fn add_package<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let path = path.as_ref().canonicalize()?;
        let manifest = Weaver::read_package_manifest(&path)?;

        self.add_package_(LocatedPackage {
            package: manifest,
            path: path,
        });
        Ok(())
    }

    fn read_package_manifest<P: AsRef<Path>>(path: P) -> io::Result<Package> {
        let mut manifest = path.as_ref().to_owned();
        manifest.push("Mod.toml");
        // TODO: there should be a check here to make sure the casing is correct.

        let mut s = String::new();
        let mut file = File::open(&manifest)?;
        file.read_to_string(&mut s)?;

        let mut parser = toml::Parser::new(&s);
        let table = match parser.parse() {
            Some(table) => table,
            None => {
                for error in &parser.errors {
                    let (line, col) = parser.to_linecol(error.lo);
                    println!("TOML parsing error: {}\nLine: {} Col: {}",
                             &error.desc,
                             line,
                             col);
                }
                let err = io::Error::new(io::ErrorKind::InvalidData, "malformed toml");
                return Err(err);
            }
        };

        let mut decoder = toml::Decoder::new(toml::Value::Table(table));
        Deserialize::deserialize(&mut decoder)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    fn add_package_(&mut self, package: LocatedPackage) {
        let version = package.package.mod_info.version.clone();
        if let Some(old_package) =
            self.packages.insert(package.package.mod_info.name.clone(), package) {
            println!("found package collision, \
                         using newest package. {}, versions: {} and {}",
                     &old_package.package.mod_info.name,
                     &version,
                     &old_package.package.mod_info.version);

            if old_package.package.mod_info.version > version {
                self.packages.insert(old_package.package.mod_info.name.clone(), old_package);
            } else if old_package.package.mod_info.version == version {
                println!("could not resolve, both version equal");
            }
        }
    }

    /// Loads a package into the game.
    ///
    /// Looks for a dylib named `{name}.module` in the
    /// root of the package folder.
    pub fn load_package(&mut self, name: &str, system: &mut ActorSystem) {
        self.load_package_(name, system);
    }

    /// Used when loading another save, and we want to reset
    /// every mod. Unloads all mods not in iter.
    ///
    /// See `Weaver::load_package` for more information.
    pub fn reset_and_load_packages<'a, I>(&mut self, iter: I, system: &mut ActorSystem)
        where I: IntoIterator<Item = &'a str>
    {
        let mut old_packs = HashMap::new();
        mem::swap(&mut old_packs, &mut self.loaded);

        for package in iter {
            if let Some(mut mod_) = old_packs.remove(package) {
                Weaver::setup_mod(&mut mod_, system);
                self.loaded.insert(package.to_owned(), mod_);
            } else {
                self.load_package_(package, system);
            }
        }
    }

    fn setup_mod(mod_: &mut LoadedMod, system: &mut ActorSystem) {
        mod_.wrapper.setup(system);
    }

    fn load_package_(&mut self, name: &str, system: &mut ActorSystem) {
        if !self.loaded.contains_key(name) {
            let package = &self.packages[name];
            let path = self.package_path(package);

            let library = Library::new(&path).unwrap();
            let mut register = Register::new();
            unsafe {
                let reg_fn: Symbol<unsafe extern "C" fn(*mut Register)> =
                    library.get(b"__register_mod\0").unwrap();
                reg_fn(&mut register as *mut Register);
            }

            if register.mods.len() != 1 {
                panic!("currently only one mod can be registered at a time. {}",
                       name);
            }

            let mod_ = register.mods.remove(0);
            let mut loaded_mod = LoadedMod {
                _library: library,
                version: package.package.mod_info.version.clone(),
                wrapper: mod_.wrapper,
            };

            Weaver::setup_mod(&mut loaded_mod, system);
            self.loaded.insert(package.package.mod_info.name.to_owned(), loaded_mod);
        }
    }

    fn package_path(&self, package: &LocatedPackage) -> PathBuf {
        const EXT: &'static str = "module";

        let name = &package.package.mod_info.name;
        let mut path = package.path.clone();

        path.push(name);
        path.set_extension(EXT);
        path
    }
}

struct LoadedMod {
    _library: Library,
    version: Version,
    wrapper: Box<ModWrapper>,
}

/// Used to register mods into the engine.
#[doc(hide)]
#[derive(Default)]
pub struct Register {
    mods: Vec<Mod>,
}

/// Used to add additional data to a mod.
#[doc(hide)]
pub struct Mod {
    wrapper: Box<ModWrapper>,
}

impl Register {
    fn new() -> Register {
        Register::default()
    }

    #[doc(hide)]
    pub fn register_mod<M>(&mut self, mod_: M) -> &mut Mod
        where M: ModWrapper + 'static
    {
        let index = self.mods.len();
        let mod_ = Mod { wrapper: Box::new(mod_) };

        self.mods.push(mod_);
        &mut self.mods[index]
    }
}
