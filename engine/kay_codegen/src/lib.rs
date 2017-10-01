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
    let traits_msgs = model.generate_trait_ids_and_messages();
    let actors_msgs = model.generate_actor_ids_messages_and_conversions();
    let setup = model.generate_setups();

    quote!(
        //! This is all auto-generated. Do not touch.
        #[allow(unused_imports)]
        use kay::{ActorSystem, ID, Fate, Actor};
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
        use kay::{ActorSystem, ID, Fate, Actor};
        use super::*;

        impl Actor for SomeActor {
            fn id(&self) -> ID {
                self.id._raw_id
            }
            unsafe fn set_id(&mut self, id: ID) {
                self.id._raw_id = id;
            }
        }

        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
        pub struct SomeActorID {
            pub _raw_id: ID
        }

        impl SomeActorID {
            pub fn local_first(world: &mut World) -> Self {
                SomeActorID { _raw_id: world.local_first::<SomeActor>() }
            }

            pub fn global_first(world: &mut World) -> Self {
                SomeActorID { _raw_id: world.global_first::<SomeActor>() }
            }

            pub fn local_broadcast(world: &mut World) -> Self {
                SomeActorID { _raw_id: world.local_broadcast::<SomeActor>() }
            }

            pub fn global_broadcast(world: &mut World) -> Self {
                SomeActorID { _raw_id: world.global_broadcast::<SomeActor>() }
            }
        }

        impl SomeActorID {
            pub fn some_method(&self, some_param: usize, world: &mut World) {
                world.send(self._raw_id, MSG_SomeActor_some_method(some_param));
            }

            pub fn no_params_fate(&self, world: &mut World) {
                world.send(self._raw_id, MSG_SomeActor_no_params_fate());
            }

            pub fn init_ish(some_param: usize, world: &mut World) -> Self {
                let id = SomeActorID { _raw_id: world.allocate_instance_id::<SomeActor>() };
                let swarm = world.local_broadcast::<SomeActor>();
                world.send(swarm, MSG_SomeActor_init_ish(id, some_param));
                id
            }
        }

        #[allow(non_camel_case_types)]
        #[derive(Compact, Clone)]
        pub struct MSG_SomeActor_some_method(pub usize);
        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone)]
        pub struct MSG_SomeActor_no_params_fate();
        #[allow(non_camel_case_types)]
        #[derive(Compact, Clone)]
        pub struct MSG_SomeActor_init_ish(pub SomeActorID, pub usize);

        #[allow(unused_variables)]
        #[allow(unused_mut)]
        pub fn auto_setup(system: &mut ActorSystem) {
            system.add_handler::<SomeActor, _, _>(|&MSG_SomeActor_some_method(ref some_param), instance, world| {
                instance.some_method(some_param, world);
                Fate::Live
            }, false);

            system.add_handler::<SomeActor, _, _>(|&MSG_SomeActor_no_params_fate(), instance, world| {
                instance.no_params_fate(world)
            }, false);

            system.add_spawner::<SomeActor, _, _>(|&MSG_SomeActor_init_ish(id, ref some_param), world| {
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

        // This shouldn't generate any ID
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
        use kay::{ActorSystem, ID, Fate, Actor};
        use super::*;

        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
        pub struct SomeTraitID {
            pub _raw_id: ID
        }

        impl SomeTraitID {
            pub fn some_method(&self, some_param: usize, world: &mut World) {
                world.send(self._raw_id, MSG_SomeTrait_some_method(some_param));
            }

            pub fn no_params_fate(&self, world: &mut World) {
                world.send(self._raw_id, MSG_SomeTrait_no_params_fate());
            }
        }

        #[allow(non_camel_case_types)]
        #[derive(Compact, Clone)]
        pub struct MSG_SomeTrait_some_method(pub usize);
        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone)]
        pub struct MSG_SomeTrait_no_params_fate();

        impl Actor for SomeActor {
            fn id(&self) -> ID {
                self.id._raw_id
            }
            unsafe fn set_id(&mut self, id: ID) {
                self.id._raw_id = id;
            }
        }

        #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
        pub struct SomeActorID {
            pub _raw_id: ID
        }

        impl SomeActorID {
            pub fn local_first(world: &mut World) -> Self {
                SomeActorID { _raw_id: world.local_first::<SomeActor>() }
            }

            pub fn global_first(world: &mut World) -> Self {
                SomeActorID { _raw_id: world.global_first::<SomeActor>() }
            }

            pub fn local_broadcast(world: &mut World) -> Self {
                SomeActorID { _raw_id: world.local_broadcast::<SomeActor>() }
            }

            pub fn global_broadcast(world: &mut World) -> Self {
                SomeActorID { _raw_id: world.global_broadcast::<SomeActor>() }
            }
        }

        impl SomeActorID { }

        impl Into<SomeTraitID> for SomeActorID {
            fn into(self) -> SomeTraitID {
                unsafe {::std::mem::transmute(self)}
            }
        }

        impl Into<ForeignTraitID> for SomeActorID {
            fn into(self) -> ForeignTraitID {
                unsafe {::std::mem::transmute(self)}
            }
        }

        #[allow(unused_variables)]
        #[allow(unused_mut)]
        pub fn auto_setup(system: &mut ActorSystem) {
            system.add_handler::<SomeActor, _, _>(|&MSG_SomeTrait_some_method(ref some_param), instance, world| {
                instance.some_method(some_param, world);
                Fate::Live
            }, false);

            system.add_handler::<SomeActor, _, _>(|&MSG_SomeTrait_no_params_fate(), instance, world| {
                instance.no_params_fate(world)
            }, false);

            system.add_handler::<SomeActor, _, _>(|&MSG_ForeignTrait_simple(ref some_param), instance, world| {
                instance.simple(some_param, world);
                Fate::Live
            }, false);
        }
    );

    assert_eq!(
        expected.into_string(),
        generate(&parse(&input.into_string()).unwrap())
    );
}
