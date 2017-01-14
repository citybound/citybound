
use std::collections::HashMap;
use std::io::{self, Read};
use std::fs::File;
use std::path::{PathBuf, Path};

use libloading::{Library, Symbol};
use kay::ActorSystem;
use serde::Deserialize;
use toml;

use ::{Package, ModWrapper};

/// The weaver takes care of loading and keeping track of mods.
#[derive(Default)]
pub struct Weaver {
    packages: HashMap<String, (PathBuf, Package)>,
    loaded: HashMap<String, LoadedMod>,
}

impl Weaver {
    pub fn new() -> Weaver {
        Weaver::default()
    }

    pub fn add_package<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let path = path.as_ref().canonicalize()?;
        let package = Weaver::read_package_manifest(&path)?;

        let old = self.packages.insert(package.ident().to_owned(), (path.clone(), package));
        if let Some(old) = old {
            println!("found package collision \"{}\"\n  {:?}\n  {:?}\n",
                     old.1.ident(),
                     &path,
                     &old.0);
        }
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
        let desc = Deserialize::deserialize(&mut decoder)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        Package::new(desc).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    /// Drops all currently loaded packages.
    pub fn drop_packages(&mut self) {
        for (_, ref mut module) in &mut self.loaded {
            module.wrapper.drop_instance();
        }
    }

    /// Loads a package into the game.
    ///
    /// `name` can either be an ident or just the name of the mod,
    /// in case of the latter, the latest version will be chosen.
    pub fn load_package(&mut self, name: &str, system: &mut ActorSystem) -> Result<(), String> {
        let ident = match self.resolve_package_ident(name) {
            Some(ident) => ident,
            None => return Err(format!("could not resolve package \"{}\"", name)),
        };

        if !self.loaded.contains_key(&ident) {
            let package = &self.packages[&ident];
            let (library, module) = self.try_load_module(package)?;
            let loaded_mod = LoadedMod {
                _library: library,
                wrapper: module.wrapper,
            };

            self.loaded.insert(ident.clone(), loaded_mod);
        }

        let module = self.loaded.get_mut(&ident).unwrap();
        module.wrapper.setup(system);
        Ok(())
    }

    fn resolve_package_ident(&self, name: &str) -> Option<String> {
        // check if name is a valid ident.
        if name.contains(':') {
            if self.packages.contains_key(name) {
                Some(name.to_owned())
            } else {
                None
            }
        } else {
            // if it isn't, find the latest version
            let mut versions = self.packages
                .iter()
                .filter(|&(n, _)| n.split(':').next().map(|n| n == name).unwrap())
                .collect::<Vec<_>>();
            versions.sort_by_key(|&(_, &(_, ref p))| p.version());
            versions.first().map(|&(n, _)| n.to_owned())
        }
    }

    fn try_load_module(&self, package: &(PathBuf, Package)) -> Result<(Library, Mod), String> {
        let path = self.module_path(package)?;

        let library = Library::new(&path).unwrap();
        let mut register = Register::new();
        unsafe {
            const FN_NAME: &'static [u8] = b"__register_mod\0";
            let reg_fn: Symbol<fn(&mut Register)> = library.get(FN_NAME).unwrap();
            reg_fn(&mut register);
        }

        if register.mods.len() != 1 {
            return Err(format!("currently, only one mod can be registered at a time. {}",
                               package.1.ident()));
        }

        Ok((library, register.mods.remove(0)))
    }

    fn module_path(&self,
                   &(ref path, ref package): &(PathBuf, Package))
                   -> Result<PathBuf, String> {
        const EXT: &'static str = "module";

        let mut path = path.clone();
        path.push(package.name());
        path.set_extension(EXT);

        if !path.exists() {
            return Err(format!("module does not exist {:?}", path));
        }
        if !path.is_file() {
            return Err(format!("module does not a file {:?}", path));
        }

        Ok(path)
    }
}

struct LoadedMod {
    _library: Library,
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
