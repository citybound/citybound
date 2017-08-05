
use kay::ActorSystem;

mod manager;
mod traits;

pub use self::manager::ModuleManager;
pub use self::traits::CityboundMod;

pub type ModuleInit = fn(&mut ActorSystem) -> Module;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ModuleVersion {
    /// API version this mod was made to be used against.
    /// This should always be the first field of this struct,
    /// so that we will always be able to find it.
    ///
    /// See `API_VERSION` for more information.
    pub api_version: *const u8,

    /// The ABI version this mod expects.
    ///
    /// See `ABI_VERSION` for more information.
    pub abi_version: *const u8,
}

impl ModuleVersion {
    pub fn verify(&self) -> Result<(), Box<Error>> {
        if ModuleVersion::convert(def.api_version)? !=
           ModuleVersion::convert(weaver::API_VERSION)? {
            return Err(format!("module api mismatch: {:?}, expected {:?}",
                               def.api_version,
                               weaver::API_VERSION)
                .into());
        }

        if ModuleVersion::convert(def.abi_version)? !=
           ModuleVersion::convert(weaver::ABI_VERSION)? {
            return Err(format!("module abi mismatch: {:?}, expected {:?}",
                               def.abi_version,
                               weaver::ABI_VERSION)
                .into());
        }

        Ok(())
    }

    fn convert<'a>(version: *const u8) -> Result<&'a str, Box<Error>> {
        use std::ffi::CStr;

        let cstr = CStr::from_ptr(version as *const i8);
        let s = cstr.to_str()?;

        Ok(s)
    }
}

#[derive(Copy, Clone)]
pub struct ModuleDef {
    /// Used to clean up anything related to handle.
    pub drop: unsafe fn(handle: *mut ()),
    pub dep_setup: unsafe fn(handle: *mut (), &package::Package, &mut ActorSystem) -> Option<Module>,
    pub dep_res: unsafe fn(handle: *mut (), &package::Package),
}

#[derive(Clone)]
pub struct Module {
    def: ModuleDef,

    /// Handle is a void ptr because we cannot assume anything about it's representation.
    handle: *mut (),
}

impl Module {
    pub fn new(def: ModuleDef, handle: *mut ()) -> Module {
        Module {
            def: def,
            handle: handle,
        }
    }

    pub unsafe fn dep_setup(&mut self,
                            package: &Package,
                            system: &mut ActorSystem)
                            -> Option<Module> {
        if self.handle.is_null() {
            return;
        }

        (self.def.dep_setup)(self.handle, package, system)
    }

    pub unsafe fn dep_res(&mut self, package: &Package) {
        if self.handle.is_null() {
            return;
        }

        (self.def.dep_res)(self.handle, package);
    }

    unsafe fn destroy(&mut self) {
        if self.handle.is_null() {
            return;
        }

        (self.def.drop)(self.handle);

        self.handle = ptr::null_mut();
    }
}

impl Drop for Module {
    fn drop(&mut self) {
        unsafe {
            self.destroy();
        }
    }
}
