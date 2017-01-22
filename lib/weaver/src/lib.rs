#![feature(plugin, conservative_impl_trait)]
#![plugin(clippy)]

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
pub use package::{Package, PackageDesc, ModInfo};
pub use weaver::{Mod, Register, Weaver, LoadingPackage};

#[macro_export]
macro_rules! register_mod {
    {
        cb_mod: $mod_:path
        $(,)*
    } => {
        #[no_mangle]
        pub fn __register_mod(register: &mut $crate::Register) {
            struct Wrapper {
                inner: Option<$mod_>,
            }

            impl $crate::ModWrapper for Wrapper {
                fn setup(&mut self, system: &mut $crate::kay::ActorSystem) {
                    let m = <$mod_ as $crate::CityboundMod>::setup(system);
                    self.inner = Some(m);
                }

                fn dependant_loading(&mut self,
                                     loading: &mut $crate::LoadingPackage,
                                     system: &mut $crate::kay::ActorSystem)
                                     -> ::std::result::Result<(), String>
                {
                    match self.inner {
                        Some(ref mut mod_) =>
                            <$mod_ as $crate::CityboundMod>::dependant_loading(mod_, loading, system),
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
