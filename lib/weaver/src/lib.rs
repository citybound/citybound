#![feature(plugin, rustc_private)]
#![plugin(clippy)]

extern crate rustc;
extern crate rustc_metadata;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate semver;
extern crate libloading;

pub extern crate kay;

mod mod_trait;
mod package;
mod weaver;

pub use mod_trait::{CityboundMod, ModWrapper};
pub use package::{Package, ModInfo};
pub use weaver::{Mod, Register, Weaver};

#[macro_export]
macro_rules! register_mod {
    {
        cb_mod: $mod_:path
        $(,)*
    } => {
        #[no_mangle]
        pub unsafe extern "C" fn __register_mod(register: *mut $crate::Register) {
            struct Wrapper {
                inner: Option<$mod_>,
            };

            impl $crate::ModWrapper for Wrapper {
                fn setup(&mut self, system: &mut $crate::kay::ActorSystem) {
                    let m = <$mod_ as $crate::CityboundMod>::setup(system);
                    self.inner = Some(m);
                }

                fn is_loaded(&mut self) -> bool {
                    self.inner.is_some()
                }
            }

            let wrapper = Wrapper {
                inner: None,
            };
            let _mod_ = (&mut *register).register_mod(wrapper);
        }
    };
}

#[cfg(test)]
mod tests {
    use ::CityboundMod;
    use kay::ActorSystem;

    #[test]
    #[allow(private_no_mangle_fns)]
    fn test_macro() {
        struct M;

        impl CityboundMod for M {
            fn setup(_system: &mut ActorSystem) -> M {
                M
            }
        }

        register_mod! {
            cb_mod: M,
        }
    }
}
