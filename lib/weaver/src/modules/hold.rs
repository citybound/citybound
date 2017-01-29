
use std::collections::HashMap;
use std::path::PathBuf;

use libloading::{Library, Symbol};

use ::{Package, ModWrapper};
use ::package::{Ident, PackageHold};
use ::modules::{Register, LoadingPackage};
use ::kay::ActorSystem;

#[derive(Default)]
pub struct ModuleHold {
    modules: HashMap<Ident, Module>,
}

impl ModuleHold {
    pub fn new() -> ModuleHold {
        ModuleHold::default()
    }

    pub fn drop_modules(&mut self) {
        for (_, ref mut module) in &mut self.modules {
            module.wrapper.drop_instance();
        }
    }

    pub fn load(&mut self,
                ident: &Ident,
                packages: &PackageHold,
                system: &mut ActorSystem)
                -> Result<(), String> {
        self.load_(ident, packages, system, &[])
            .map(|_| ())
    }

    fn load_<'a>(&'a mut self,
                 ident: &Ident,
                 packages: &PackageHold,
                 system: &mut ActorSystem,
                 parents: &[&Ident])
                 -> Result<&'a mut Module, String> {
        if parents.contains(&ident) {
            return Err(format!("dependency loop, {:?}", parents));
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
                module.wrapper.dependant_loading(&mut loading, system)?;
            }

            let module = ModuleHold::try_load_module(&loading)?;
            self.modules.insert(ident.to_owned(), module);
        }

        let module = self.modules.get_mut(ident).unwrap();
        if !module.wrapper.has_instance() {
            module.wrapper.setup(system);
        }
        Ok(module)
    }

    fn try_load_module(package: &LoadingPackage) -> Result<Module, String> {
        let library = Library::new(package.module_path()).unwrap();
        let mut register = Register::new();
        unsafe {
            const FN_NAME: &'static [u8] = b"__register_mod\0";
            let reg_fn: Symbol<fn(&mut Register)> = library.get(FN_NAME).unwrap();
            reg_fn(&mut register);
        }

        let (mut mods,) = register.deconstruct();
        if mods.len() != 1 {
            return Err(format!("currently, only one mod can be registered at a time, '{}'",
                               package.package().ident()));
        }

        let (wrapper,) = mods.remove(0).deconstruct();
        Ok(Module {
            wrapper: wrapper,
            _library: library,
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

// Please note, that the order of drops here is extremely important!
// Things loaded from the library **must** be dropped before the
// library itself. Otherwise the instructions to do the drop might
// be unloaded, causing segfaults and other weird errors.
//
// Consider to manually implement the `Drop` trait.
struct Module {
    wrapper: Box<ModWrapper>,
    _library: Library,
}
