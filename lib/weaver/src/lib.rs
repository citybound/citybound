#![feature(conservative_impl_trait)]
#![feature(slice_patterns)]
#![feature(plugin)]
#![plugin(clippy)]

//! Crate for loading and defining mods.
//!
//! ```
//! # #[macro_use] extern crate weaver;
//! # use weaver::CityboundMod;
//! # use weaver::kay::ActorSystem;
//! #
//! struct MyMod;
//!
//! impl CityboundMod for MyMod {
//!    fn setup(_system: &mut ActorSystem) -> MyMod {
//!        // todo: setup my mod using the actor system.
//!        MyMod
//!    }
//! }
//!
//! register_module! {
//!     module: MyMod,
//! }
//! #
//! # // make rustdoc happy
//! # fn main() {}
//! ```

extern crate libloading;
#[macro_use]
extern crate lazy_static;
extern crate rayon;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate semver;
extern crate toml;

pub extern crate kay;

pub mod packages;
pub mod modules;
mod weaver;

pub use modules::CityboundMod;
pub use weaver::Weaver;

/// Version of weaver. Used to verify API compatibility.
pub const API_VERSION: &'static [u8] = &b"v0.1\0"[..];

/// Version of rustc used to compile weaver. Used to verify ABI compatibility.
pub const ABI_VERSION: &'static [u8] = include!(concat!(env!("OUT_DIR"), "/rustc_version.rs"));

/// This macro helps to create the right functions for registering
/// this crate as a mod when the crate is loaded by weaver.
#[macro_export]
macro_rules! register_module {
    {
        module: $mod_:path,
    } => {
        #[no_mangle]
        #[doc(hidden)]
        pub static __MODULE_VERSION: $crate::ModuleVersion = {
            api_version: $crate::API_VERSION,
            abi_version: $crate::ABI_VERSION,
        };

        #[no_mangle]
        #[doc(hidden)]
        pub fn __module_setup(system: &mut $crate::kay::ActorSystem) -> $crate::Module {
            unsafe fn module_drop(handle: *mut ()) {
                Box::from_raw(handle as *mut $mod_);
            }

            unsafe fn module_dep_setup(handle: *mut (),
                                       package: &$crate::Package,
                                       system: &mut $crate::kay::ActorSystem)
                                       -> Option<Module>
            {
                let handle = (handle as *mut $mod_).as_mut().expect("handle was null");
                handle.dep_setup(package, system).unwrap();
            }

            // Can we let unwinds/panics pass over the library boundry?
            fn _module_print_unwind(error: Box<std::any::Any>) {
                match error.downcast::<String>() {
                    Ok(string) => println!("{:?}", string),
                    Err(any) => {
                        match any.downcast::<&'static str>() {
                            Ok(string) => println!("{:?}", string),
                            Err(_) => println!("Weird error type"),
                        }
                    }
                }
            }

            let def = $crate::ModuleDef {
                drop: module_drop,
                dep_setup: dep_setup,
                dep_res: dep_res,
            };

            let module = <$mod_ as $crate::CityboundMod>::setup(system);
            let handle = Box::into_raw(Box::new(module)) as *mut ();

            $crate::Module::new(def, handle)
        }
    };
}

#[cfg(test)]
#[allow(private_no_mangle_fns)]
mod tests {
    use super::CityboundMod;
    use super::kay::ActorSystem;

    struct MyMod;

    impl CityboundMod for MyMod {
        fn setup(_system: &mut ActorSystem) -> MyMod {
            MyMod
        }
    }

    register_module! {
        module: MyMod,
    }
}
