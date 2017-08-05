
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Arc, Mutex};
use std::error::Error;
use std::path::PathBuf;
use std::ptr;

use libloading::{Library, Symbol};

use rayon::prelude::*;

use packages::{Ident, Package, PackageManager, ResolvedPackage};
use modules::{Module, ModuleDef};
use kay::ActorSystem;

#[derive(Default)]
pub struct ModuleManager {
    modules: Vec<Module>,
}

impl ModuleManager {
    pub fn new(packages: HashSet<ResolvedPackage>, system: &mut ActorSystem) -> ModuleManager {
        let mut dependents: HashMap<Ident, Vec<Ident>> = HashMap::new();
        let mut load_queue: HashMap<Ident, AtomicIsize> = HashMap::new();
        let mut modules: HashMap<Ident, Vec<Module>> = HashMap::new();
        let mut starting = Vec::new();

        for package in &packages {
            match &package {
                ResolvedPackage::External(pkg, _) if pkg.dependencies.len() == 0 => {
                    starting.push(pkg.ident);
                }
                ResolvedPackage::External(pkg, path) => {}
                ResolvedPackage::Builtin(pkg, def) => {
                    starting.push(pkg.ident);

                    let module = unsafe { Module::setup(def) };
                }
            }
        }

        let system = Mutex::new(system);
        let modules: Mutex<HashMap<Ident, Vec<Module>>> = Mutex::new(modules);
        let loaded_modules: Mutex<Vec<Module>> = Mutex::new(Vec::new());

        let load_module = |ident| {
            let deps = dependencies.get(ident);
            let modules = modules.lock().unwrap().remove(ident).unwrap();

            for module in &modules {
                module.dependant_loading();
            }
        };

        starting.par_iter().for_each(load_module);

        let modules = modules.into_inner().unwrap();

        ModuleManager { modules: modules }
    }

    pub fn load(&mut self,
                ident: &Ident,
                packages: &PackageHold,
                system: &mut ActorSystem)
                -> Result<(), Box<Error>> {
        self.load_(ident, packages, system, &[]).map(|_| ())
    }

    fn load_<'a>(&'a mut self,
                 ident: &Ident,
                 packages: &PackageHold,
                 system: &mut ActorSystem,
                 parents: &[&Ident])
                 -> Result<&'a mut Module, Box<Error>> {
        if parents.contains(&ident) {
            return Err(format!("dependency loop, {:?}", parents).into());
        }

        let parents = {
            let mut p = parents.to_vec();
            p.push(ident);
            p
        };

        if !self.modules.contains_key(ident) {
            let package = packages.get(ident)
                .expect(&format!("couldn't find '{}'", ident));
            let path = ModuleHold::module_path(package)?;
            let mut loading = LoadingPackage::new(package, path);

            for dep in package.dependencies() {
                let module = self.load_(dep.ident(), packages, system, &parents)?;
                module.dependant_loading(&mut loading, system);
            }

            let module = ModuleHold::try_load_module(&loading)?;
            self.modules.insert(ident.to_owned(), module);
        }

        let module = self.modules.get_mut(ident).unwrap();
        module.setup(system);

        Ok(module)
    }

    fn try_load_module(package: &LoadingPackage) -> Result<Module, String> {
        let library = Library::new(package.module_path()).unwrap();

        Ok(Module {
            library: library,
            handle: ptr::null_mut(),
        })
    }

    fn module_path(package: &Package) -> Result<PathBuf, String> {
        const EXT: &'static str = "module";

        let mut path = package.path().to_owned();
        path.push(package.ident().name());
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
