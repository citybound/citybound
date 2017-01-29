#![feature(plugin, conservative_impl_trait)]
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
//! register_mod! {
//!     cb_mod: MyMod,
//! }
//! #
//! # // make rustdoc happy
//! # fn main() {}
//! ```

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate semver;
extern crate libloading;

pub extern crate kay;

pub mod package;
pub mod modules;
mod weaver;

pub use modules::{CityboundMod, ModWrapper};
pub use package::Package;
pub use weaver::Weaver;

/// This macro helps to create the right functions for registering
/// this crate as a mod when the crate is loaded by weaver.
#[macro_export]
macro_rules! register_mod {
    {
        cb_mod: $mod_:path
        $(,)*
    } => {
        #[no_mangle]
        #[doc(hidden)]
        pub fn __register_mod(register: &mut $crate::modules::Register) {
            struct Wrapper {
                inner: Option<$mod_>,
            }

            impl $crate::ModWrapper for Wrapper {
                fn setup(&mut self, system: &mut $crate::kay::ActorSystem) {
                    let m = <$mod_ as $crate::CityboundMod>::setup(system);
                    self.inner = Some(m);
                }

                fn dependant_loading(&mut self,
                                     loading: &mut $crate::modules::LoadingPackage,
                                     system: &mut $crate::kay::ActorSystem)
                                     -> ::std::result::Result<(), String>
                {
                    match self.inner {
                        Some(ref mut mod_) =>
                            <$mod_ as $crate::CityboundMod>::dependant_loading(mod_,
                                                                               loading,
                                                                               system),
                        None => panic!("mod not loaded"),
                    }
                }

                fn has_instance(&mut self) -> bool {
                    self.inner.is_some()
                }

                fn drop_instance(&mut self) {
                    self.inner = None;
                }
            }

            let wrapper = Wrapper {
                inner: None,
            };
            let _mod_ = register.register_mod(wrapper);
        }
    };
}
