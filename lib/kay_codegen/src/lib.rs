#![recursion_limit="128"]
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

pub fn scan_and_generate() {
    for maybe_mod_path in glob("src/**/mod.rs").unwrap() {
        if let Ok(mod_path) = maybe_mod_path {
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
                        generate(&parse(&file_str))
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
            }
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
    critical: bool,
    returns_fate: bool,
    from_trait: Option<TraitName>,
}

pub fn generate(model: &Model) -> String {
    let traits_msgs = model.generate_trait_ids_and_messages();
    let actors_msgs = model.generate_actor_ids_messages_and_conversions();
    let setup = model.generate_setups();

    quote!(
        //! This is all auto-generated. Do not touch.
        use kay::ActorSystem;
        use kay::swarm::Swarm;
        use super::*;

        #traits_msgs
        #actors_msgs
        pub fn auto_setup(system: &mut ActorSystem) {
            #setup
        }

    ).into_string()
}

#[test]
fn simple_actor() {
    let input = quote!(
        pub struct SomeActor {
            _id: Option<SomeActorID>,
            field: usize
        }

        impl SomeActor {
            pub fn some_method(&mut self, some_param: &usize, world: &mut World) {
                self.id().some_method(42, world);
            }

            pub fn no_params_fate(&mut self, world: &mut World) -> Fate {
                Fate::Die
            }
        }
    );
    let expected = quote!(
        //! This is all auto-generated. Do not touch.
        use kay::ActorSystem;
        use kay::swarm::Swarm;
        use super::*;

        #[derive(Copy, Clone)]
        pub struct SomeActorID {
            pub _raw_id: ID
        }

        impl SomeActorID {
            pub fn in_world(world: &mut World) -> Self {
                SomeActorID { _raw_id: world.id::<Swarm<SomeActor>>() }
            }
        }

        impl SomeActorID {
            pub fn some_method(&self, some_param: usize, world: &mut World) {
                world.send(self._raw_id, MSG_SomeActor_some_method(some_param));
            }

            pub fn no_params_fate(&self, world: &mut World) {
                world.send(self._raw_id, MSG_SomeActor_no_params_fate());
            }
        }

        #[allow(non_camel_case_types)]
        #[derive(Compact, Clone)]
        struct MSG_SomeActor_some_method(usize);
        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone)]
        struct MSG_SomeActor_no_params_fate();

        pub fn auto_setup(system: &mut ActorSystem) {
            system.extend::<Swarm<SomeActor>, _>(Swarm::<SomeActor>::subactors(|mut definer| {
                definer.on(|&MSG_SomeActor_some_method(ref some_param), actor, world| {
                    actor.some_method(some_param, world);
                    Fate::Live
                });
                definer.on(|&MSG_SomeActor_no_params_fate(), actor, world| {
                    actor.no_params_fate(world)
                });
            }));
        }
    );

    assert_eq!(expected.into_string(), generate(&parse(&input.into_string())));
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
    );
    let expected = quote!(
        //! This is all auto-generated. Do not touch.
        use kay::ActorSystem;
        use kay::swarm::Swarm;
        use super::*;

        #[derive(Copy, Clone)]
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
        struct MSG_SomeTrait_some_method(usize);
        #[allow(non_camel_case_types)]
        #[derive(Copy, Clone)]
        struct MSG_SomeTrait_no_params_fate();

        #[derive(Copy, Clone)]
        pub struct SomeActorID {
            pub _raw_id: ID
        }

        impl SomeActorID {
            pub fn in_world(world: &mut World) -> Self {
                SomeActorID { _raw_id: world.id::<Swarm<SomeActor>>() }
            }
        }

        impl SomeActorID { }

        impl Into<SomeTraitID> for SomeActorID {
            fn into(self) -> SomeTraitID {
                unsafe {::std::mem::transmute(self)}
            }
        }

        pub fn auto_setup(system: &mut ActorSystem) {
            system.extend::<Swarm<SomeActor>, _>(Swarm::<SomeActor>::subactors(|mut definer| {
                definer.on(|&MSG_SomeTrait_some_method(ref some_param), actor, world| {
                    actor.some_method(some_param, world);
                    Fate::Live
                });
                definer.on(|&MSG_SomeTrait_no_params_fate(), actor, world| {
                    actor.no_params_fate(world)
                });
            }));
        }
    );

    assert_eq!(expected.into_string(), generate(&parse(&input.into_string())));
}