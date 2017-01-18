
use std::collections::{HashMap, HashSet};
use std::io::{self, Read};
use std::fs::{self, File};
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
    loaded: HashMap<String, LoadedModule>,
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
            println!("found package collision '{}'\n  {:?}\n  {:?}\n",
                     old.1.ident(),
                     &path,
                     &old.0);
        }
        Ok(())
    }

    pub fn add_folder<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        for dir in fs::read_dir(path)? {
            let path = dir?.path();
            if Weaver::is_mod_dir(&path) {
                self.add_package(path)?;
            } else {
                println!("no Mod.toml found in {:?}", path);
            }
        }
        Ok(())
    }

    fn is_mod_dir<P: AsRef<Path>>(path: P) -> bool {
        let mut manifest = path.as_ref().to_owned();
        manifest.push("Mod.toml");
        manifest.is_file()
    }

    fn read_package_manifest<P: AsRef<Path>>(path: P) -> io::Result<Package> {
        let mut manifest = path.as_ref().to_owned();
        manifest.push("Mod.toml");
        // TODO: there should be a check here to make sure the casing is correct.

        let mut s = String::new();
        let mut file = File::open(&manifest)?;
        file.read_to_string(&mut s)?;

        let mut parser = toml::Parser::new(&s);
        let table;
        match parser.parse() {
            Some(t) => table = t,
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
        }

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
        let ident;
        match Weaver::resolve_package_ident(name, &self.packages) {
            Some(i) => ident = i,
            None => return Err(format!("could not resolve package '{}'", name)),
        }

        let mut loaded = HashSet::new();
        Weaver::load_package_(&ident,
                              &self.packages,
                              &mut self.loaded,
                              system,
                              &mut loaded)?;
        Ok(())
    }

    fn load_package_<'a>(ident: &str,
                         packages: &HashMap<String, (PathBuf, Package)>,
                         loaded: &'a mut HashMap<String, LoadedModule>,
                         system: &mut ActorSystem,
                         parents: &mut HashSet<String>)
                         -> Result<&'a mut LoadedModule, String> {
        // This is a very rough implementation of cycle detection.
        // Maybe we can keep track of parents instead?
        if parents.contains(ident) {
            return Err(format!("found cyclic dependency: '{}'", ident));
        }
        parents.insert(ident.to_owned());

        if !loaded.contains_key(ident) {
            let package = &packages[ident];
            let path = Weaver::module_path(package)?;
            let mut loading = LoadingPackage {
                ident: ident,
                path: &package.0,
                package: &package.1,
                module_path: path,
            };

            for (name, _dep) in package.1.dependencies() {
                // TODO: do ident resolution with version as well.
                let ident;
                match Weaver::resolve_package_ident(name, packages) {
                    Some(i) => ident = i,
                    None => return Err(format!("could not resolve package '{}'", name)),
                }

                let module = Weaver::load_package_(&ident, packages, loaded, system, parents)?;
                module.wrapper.dependant_loading(&mut loading, system)?;
            }

            let module = Weaver::try_load_module(&loading)?;
            loaded.insert(ident.to_owned(), module);
        }

        let module = loaded.get_mut(ident).unwrap();
        if !module.wrapper.has_instance() {
            module.wrapper.setup(system);
        }
        Ok(module)
    }

    fn resolve_package_ident(name: &str,
                             packages: &HashMap<String, (PathBuf, Package)>)
                             -> Option<String> {
        // check if name is a valid ident.
        if name.contains(':') {
            if packages.contains_key(name) {
                Some(name.to_owned())
            } else {
                None
            }
        } else {
            // if it isn't, find the latest version
            let mut versions = packages.iter()
                .filter(|&(n, _)| n.split(':').next().map(|n| n == name).unwrap())
                .collect::<Vec<_>>();
            versions.sort_by_key(|&(_, &(_, ref p))| p.version());
            versions.first().map(|&(n, _)| n.to_owned())
        }
    }

    fn try_load_module(package: &LoadingPackage) -> Result<LoadedModule, String> {
        let library = Library::new(&package.module_path).unwrap();
        let mut register = Register::new();
        unsafe {
            const FN_NAME: &'static [u8] = b"__register_mod\0";
            let reg_fn: Symbol<fn(&mut Register)> = library.get(FN_NAME).unwrap();
            reg_fn(&mut register);
        }

        if register.mods.len() != 1 {
            return Err(format!("currently, only one mod can be registered at a time, '{}'",
                               package.package.ident()));
        }

        let module = register.mods.remove(0);
        Ok(LoadedModule {
            wrapper: module.wrapper,
            _library: library,
        })
    }

    fn module_path(&(ref path, ref package): &(PathBuf, Package)) -> Result<PathBuf, String> {
        const EXT: &'static str = "module";

        let mut path = path.clone();
        path.push(package.name());
        path.set_extension(EXT);

        if !path.exists() {
            return Err(format!("module does not exist {:?}", path));
        }
        if !path.is_file() {
            return Err(format!("module is not a file {:?}", path));
        }

        Ok(path)
    }
}

/// This is the description sent to dependants of this package,
/// before the module is actually loaded.
pub struct LoadingPackage<'a> {
    ident: &'a str,
    path: &'a Path,
    package: &'a Package,
    module_path: PathBuf,
}

impl<'a> LoadingPackage<'a> {
    #[inline]
    pub fn ident(&self) -> &str {
        self.ident
    }

    #[inline]
    pub fn path(&self) -> &Path {
        self.path
    }

    #[inline]
    pub fn package(&self) -> &Package {
        self.package
    }

    #[inline]
    pub fn set_module_path<P: AsRef<Path>>(&mut self, path: P) {
        self.module_path = path.as_ref().to_owned();
    }
}

// Please note, that the order of drops here is extremely important!
// Things loaded from the library **must** be dropped before the
// library itself. Otherwise the instructions to do the drop might
// be unloaded, causing segfaults and other weird errors.
//
// Consider to manually implement the `Drop` trait.
struct LoadedModule {
    wrapper: Box<ModWrapper>,
    _library: Library,
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
