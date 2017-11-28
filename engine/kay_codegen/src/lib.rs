#![recursion_limit="256"]
#![feature(conservative_impl_trait)]

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

extern crate syn;
#[macro_use]
extern crate quote;
use syn::*;
extern crate glob;
use glob::glob;

use std::fs::{File, metadata};
use std::io::Read;
use std::io::Write;
use std::process::Command;

extern crate ordermap;
use ordermap::OrderMap;

mod generators;
mod parsers;
use parsers::parse;

pub fn scan_and_generate(src_prefix: &str) {
    for maybe_mod_path in glob(&format!("{}/**/mod.rs", src_prefix)).unwrap() {
        if let Ok(mod_path) = maybe_mod_path {
            //println!("cargo:warning={:?}", mod_path);
            let auto_path = mod_path.clone().to_str().unwrap().replace(
                "mod.rs",
                "kay_auto.rs",
            );
            if let Ok(src_meta) = metadata(&mod_path) {
                let regenerate = match metadata(&auto_path) {
                    Ok(auto_meta) => src_meta.modified().unwrap() > auto_meta.modified().unwrap(),
                    _ => true,
                };

                if regenerate {
                    let auto_file = if let Ok(ref mut file) = File::open(&mod_path) {
                        let mut file_str = String::new();
                        file.read_to_string(&mut file_str).unwrap();
                        match parse(&file_str) {
                            Ok(model) => generate(&model),
                            Err(error) => format!("PARSE ERROR:\n {:?}", error),
                        }
                    } else {
                        panic!("couldn't load");
                    };

                    if let Ok(ref mut file) = File::create(&auto_path) {
                        file.write_all(auto_file.as_bytes()).unwrap();
                    }

                    let _ = Command::new("rustfmt")
                        .arg("--write-mode")
                        .arg("overwrite")
                        .arg(&auto_path)
                        .spawn();
                }
            } else {
                panic!("couldn't load");
            };
        }
    }
}

type ActorName = Ty;
type TraitName = syn::Path;

#[derive(Default)]
pub struct Model {
    pub actors: OrderMap<ActorName, ActorDef>,
    pub traits: OrderMap<TraitName, TraitDef>,
}

#[derive(Default)]
pub struct ActorDef {
    pub handlers: Vec<Handler>,
    pub impls: Vec<TraitName>,
    pub defined_here: bool,
}

#[derive(Default)]
pub struct TraitDef {
    pub handlers: Vec<Handler>,
}

#[derive(Clone)]
pub struct Handler {
    name: Ident,
    arguments: Vec<FnArg>,
    scope: HandlerType,
    critical: bool,
    returns_fate: bool,
    from_trait: Option<TraitName>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum HandlerType {
    Handler,
    Init,
}

pub fn generate(model: &Model) -> String {
    let traits_msgs = model.generate_traits();
    let actors_msgs = model.generate_actor_ids_messages_and_conversions();
    let setup = model.generate_setups();

    quote!(
        //! This is all auto-generated. Do not touch.
        #[allow(unused_imports)]
        use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom};
        use super::*;

        #traits_msgs
        #actors_msgs

        #[allow(unused_variables)]
        #[allow(unused_mut)]
        pub fn auto_setup(system: &mut ActorSystem) {
            #setup
        }

    ).into_string()
}

#[test]
fn simple_actor() {
    let input = quote!(
        pub struct SomeActor {
            id: Option<SomeActorID>,
            field: usize
        }

        impl SomeActor {
            pub fn some_method(&mut self, some_param: &usize, world: &mut World) {
                self.id().some_method(42, world);
            }

            pub fn no_params_fate(&mut self, world: &mut World) -> Fate {
                Fate::Die
            }

            pub fn init_ish(id: SomeActorID, some_param: &usize, world: &mut World) -> SomeActor {
                SomeActor {
                    id: Some(id),
                    field: some_param
                }
            }
        }
    );
    let expected = quote!(
        //! This is all auto-generated. Do not touch.
        #[allow(unused_imports)]
        use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom};
        use super::*;

        impl Actor for SomeActor {
            type ID = SomeActorID;

            fn id(&self) -> Self::ID {
                self.id
            }
            unsafe fn set_id(&mut self, id: RawID) {
                self.id = Self::ID::from_raw(id);
            }
        }

        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
        pub struct SomeActorID {
            _raw_id: RawID
        }

        impl TypedID for SomeActorID {
            unsafe fn from_raw(id: RawID) -> Self {
                SomeActorID { _raw_id: id }
            }

            fn as_raw(&self) -> RawID {
                self._raw_id
            }
        }

        impl SomeActorID {
            pub fn some_method(&self, some_param: usize, world: &mut World) {
                world.send(self.as_raw(), MSG_SomeActor_some_method(some_param));
            }

            pub fn no_params_fate(&self, world: &mut World) {
                world.send(self.as_raw(), MSG_SomeActor_no_params_fate());
            }

            pub fn init_ish(some_param: usize, world: &mut World) -> Self {
                let id = unsafe{
                    SomeActorID::from_raw(world.allocate_instance_id::<SomeActor>())
                };
                let swarm = world.local_broadcast::<SomeActor>();
                world.send(swarm, MSG_SomeActor_init_ish(id, some_param));
                id
            }
        }

        #[allow(non_camel_case_types)]
        #[derive(Compact, Clone)]
        struct MSG_SomeActor_some_method(pub usize);
        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone)]
        struct MSG_SomeActor_no_params_fate();
        #[allow(non_camel_case_types)]
        #[derive(Compact, Clone)]
        struct MSG_SomeActor_init_ish(pub SomeActorID, pub usize);

        #[allow(unused_variables)]
        #[allow(unused_mut)]
        pub fn auto_setup(system: &mut ActorSystem) {
            system.add_handler::<SomeActor, _, _>(
                |&MSG_SomeActor_some_method(ref some_param), instance, world| {
                instance.some_method(some_param, world);
                Fate::Live
            }, false);

            system.add_handler::<SomeActor, _, _>(
                |&MSG_SomeActor_no_params_fate(), instance, world| {
                instance.no_params_fate(world)
            }, false);

            system.add_spawner::<SomeActor, _, _>(
                |&MSG_SomeActor_init_ish(id, ref some_param), world| {
                SomeActor::init_ish(id, some_param, world)
            }, false);
        }
    );

    assert_eq!(
        expected.into_string(),
        generate(&parse(&input.into_string()).unwrap())
    );
}

#[test]
fn trait_and_impl() {
    let input = quote!(
        pub struct SomeActor {
            _id: Option<SomeActorID>,
            field: usize
        }

        trait SomeTrait {
            fn some_method(&mut self, some_param: &usize, world: &mut World);
            fn no_params_fate(&mut self, world: &mut World) -> Fate;
            fn some_default_impl_method(&mut self, world: &mut World) {
                self.some_method(3, world);
            }
        }

        impl SomeTrait for SomeActor {
            fn some_method(&mut self, some_param: &usize, world: &mut World) {
                self.id().some_method(42, world);
            }

            fn no_params_fate(&mut self, world: &mut World) -> Fate {
                Fate::Die
            }
        }

        impl ForeignTrait for SomeActor {
            fn simple(&mut self, some_param: &usize, world: &mut World) {
                self.id().some_method(some_param, world);
            }
        }

        // This shouldn't generate any RawID
        impl Deref for SomeActor {
            type Target = usize;
            fn deref(&self) -> &usize {
                &self.field
            }
        }
    );
    let expected = quote!(
        //! This is all auto-generated. Do not touch.
        #[allow(unused_imports)]
        use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom};
        use super::*;

        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
        pub struct SomeTraitID {
            _raw_id: RawID
        }

        impl TypedID for SomeTraitID {
            unsafe fn from_raw(id: RawID) -> Self {
                SomeTraitID { _raw_id: id }
            }

            fn as_raw(&self) -> RawID {
                self._raw_id
            }
        }

        impl<A: Actor + SomeTrait> TraitIDFrom<A> for SomeTraitID {}

        impl SomeTraitID {
            pub fn some_method(&self, some_param: usize, world: &mut World) {
                world.send(self.as_raw(), MSG_SomeTrait_some_method(some_param));
            }

            pub fn no_params_fate(&self, world: &mut World) {
                world.send(self.as_raw(), MSG_SomeTrait_no_params_fate());
            }

            pub fn some_default_impl_method(&self, world: &mut World) {
                world.send(self.as_raw(), MSG_SomeTrait_some_default_impl_method());
            }

            pub fn register_handlers<A: Actor + SomeTrait>(system: &mut ActorSystem) {
                system.add_handler::<A, _, _>(
                    |&MSG_SomeTrait_some_method(ref some_param), instance, world| {
                    instance.some_method(some_param, world);
                    Fate::Live
                }, false);

                system.add_handler::<A, _, _>(
                    |&MSG_SomeTrait_no_params_fate(), instance, world| {
                    instance.no_params_fate(world)
                }, false);

                system.add_handler::<A, _, _>(
                    |&MSG_SomeTrait_some_default_impl_method(), instance, world| {
                    instance.some_default_impl_method(world);
                    Fate::Live
                }, false);
            }
        }

        #[allow(non_camel_case_types)]
        #[derive(Compact, Clone)]
        struct MSG_SomeTrait_some_method(pub usize);
        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone)]
        struct MSG_SomeTrait_no_params_fate();
        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone)]
        struct MSG_SomeTrait_some_default_impl_method();

        impl Actor for SomeActor {
            type ID = SomeActorID;

            fn id(&self) -> Self::ID {
                self.id
            }
            unsafe fn set_id(&mut self, id: RawID) {
                self.id = Self::ID::from_raw(id);
            }
        }

        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
        pub struct SomeActorID {
            _raw_id: RawID
        }

        impl TypedID for SomeActorID {
            unsafe fn from_raw(id: RawID) -> Self {
                SomeActorID { _raw_id: id }
            }

            fn as_raw(&self) -> RawID {
                self._raw_id
            }
        }

        impl SomeActorID { }

        #[allow(unused_variables)]
        #[allow(unused_mut)]
        pub fn auto_setup(system: &mut ActorSystem) {
            SomeTraitID::register_handlers::<SomeActor>(system);
            ForeignTraitID::register_handlers::<SomeActor>(system);
        }
    );

    assert_eq!(
        expected.into_string(),
        generate(&parse(&input.into_string()).unwrap())
    );
}
