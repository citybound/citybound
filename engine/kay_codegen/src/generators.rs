use syn::*;
use quote::Tokens;
use {Model, ActorName, TraitName, Handler};
use BindingMode::{ByRef, ByValue};
use Mutability::Immutable;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum HandlerOrigin {
    OnlyOwn,
    AlsoFromTraits,
}
use self::HandlerOrigin::*;

use {HandlerScope, ActorDef, TraitDef};

impl ActorDef {
    pub fn map_handlers<O, F>(
        &self,
        name: &ActorName,
        origin: HandlerOrigin,
        scope: HandlerScope,
        map_f: F,
    ) -> Vec<O>
    where
        F: Fn(&ActorName, &Handler) -> O,
    {
        self.handlers
            .iter()
            .filter(|&handler| {
                (origin == AlsoFromTraits || handler.from_trait.is_none()) &&
                    (handler.scope == scope)
            })
            .map(|handler| map_f(name, handler))
            .collect()
    }
}

impl TraitDef {
    pub fn map_handlers<O, F>(&self, name: &TraitName, scope: HandlerScope, map_f: F) -> Vec<O>
    where
        F: Fn(&TraitName, &Handler) -> O,
    {
        self.handlers
            .iter()
            .filter(|&handler| handler.scope == scope)
            .map(|handler| map_f(name, handler))
            .collect()
    }
}

impl Model {
    pub fn map_handlers<O, F>(
        &self,
        origin: HandlerOrigin,
        map_f: F,
    ) -> (Vec<Vec<O>>, Vec<Vec<O>>, Vec<Vec<O>>)
    where
        F: Fn(&ActorName, &Handler) -> O,
    {
        let mut for_subactor = Vec::<Vec<O>>::new();
        let mut for_swarm = Vec::<Vec<O>>::new();
        let mut for_init = Vec::<Vec<O>>::new();

        for (actor_name, actor_def) in self.actors.iter() {
            for_subactor.push(actor_def.map_handlers(
                actor_name,
                origin,
                HandlerScope::SubActor,
                &map_f,
            ));
            for_swarm.push(actor_def.map_handlers(
                actor_name,
                origin,
                HandlerScope::Swarm,
                &map_f,
            ));
            for_init.push(actor_def.map_handlers(
                actor_name,
                origin,
                HandlerScope::Init,
                &map_f,
            ));
        }

        (for_subactor, for_swarm, for_init)
    }

    pub fn map_handlers_args<O, F>(
        &self,
        origin: HandlerOrigin,
        map_f: F,
    ) -> (Vec<Vec<Vec<O>>>, Vec<Vec<Vec<O>>>, Vec<Vec<Vec<O>>>)
    where
        F: Fn(&FnArg) -> O,
    {
        self.map_handlers(origin, |_, handler| {
            handler.arguments.iter().map(&map_f).collect()
        })
    }

    pub fn map_trait_handlers<O, F>(&self, map_f: F) -> (Vec<Vec<O>>, Vec<Vec<O>>, Vec<Vec<O>>)
    where
        F: Fn(&TraitName, &Handler) -> O,
    {
        let mut for_subactor = Vec::<Vec<O>>::new();
        let mut for_swarm = Vec::<Vec<O>>::new();
        let mut for_init = Vec::<Vec<O>>::new();

        for (trait_name, trait_def) in self.traits.iter() {
            for_subactor.push(trait_def.map_handlers(
                trait_name,
                HandlerScope::SubActor,
                &map_f,
            ));
            for_swarm.push(trait_def.map_handlers(
                trait_name,
                HandlerScope::Swarm,
                &map_f,
            ));
            for_init.push(trait_def.map_handlers(
                trait_name,
                HandlerScope::Init,
                &map_f,
            ));
        }

        (for_subactor, for_swarm, for_init)
    }

    pub fn map_trait_handlers_args<O, F>(
        &self,
        map_f: F,
    ) -> (Vec<Vec<Vec<O>>>, Vec<Vec<Vec<O>>>, Vec<Vec<Vec<O>>>)
    where
        F: Fn(&FnArg) -> O,
    {
        self.map_trait_handlers(|_, handler| handler.arguments.iter().map(&map_f).collect())
    }

    pub fn generate_setups(&self) -> Tokens {
        let setup_types_1: Vec<_> = self.actors.iter().map(|(ty, _)| ty).collect();
        let (setup_types_2, setup_types_3) = (setup_types_1.clone(), setup_types_1.clone());
        let (handler_mods, swarm_handler_mods, init_handler_mods) =
            self.map_handlers(AlsoFromTraits, |_, handler| {
                Ident::from(if handler.critical {
                    "on_critical"
                } else {
                    "on"
                })
            });
        let (msg_names, swarm_msg_names, init_msg_names) =
            self.map_handlers(AlsoFromTraits, |actor_name, handler| {
                let msg_prefix = typ_to_message_prefix(actor_name, handler.from_trait.as_ref());
                Ident::new(format!("{}_{}", msg_prefix, handler.name))
            });
        let (msg_args, swarm_msg_args, init_msg_args) =
            self.map_handlers_args(AlsoFromTraits, |arg| match arg {
                &FnArg::Captured(Pat::Ident(_, ref ident, _), ref ty) => {
                    match ty {
                        &Ty::Rptr(_, _) => Pat::Ident(ByRef(Immutable), ident.clone(), None),
                        _ => Pat::Ident(ByValue(Immutable), ident.clone(), None),
                    }
                }
                _ => unimplemented!(),
            });
        let (handler_names, swarm_handler_names, init_handler_names) =
            self.map_handlers(AlsoFromTraits, |_, handler| handler.name.clone());
        let (handler_params, swarm_handler_params, init_handler_params) =
            self.map_handlers_args(AlsoFromTraits, |arg| match arg {
                &FnArg::Captured(Pat::Ident(_, ref ident, _), _) => ident.clone(),
                _ => unimplemented!(),
            });
        let (maybe_fate_returns, swarm_maybe_fate_returns, _) =
            self.map_handlers(AlsoFromTraits, |_, handler| if handler.returns_fate {
                quote!()
            } else {
                quote!(; Fate::Live)
            });
        let (_, types_for_swarm_handlers, types_for_init_handlers_1) =
            self.map_handlers(AlsoFromTraits, |actor_name, _| actor_name.clone());
        let types_for_init_handlers_2 = types_for_init_handlers_1.clone();

        quote!(
            #(
                system.extend::<Swarm<#setup_types_1>, _>(
                    Swarm::<#setup_types_2>::subactors(|mut each_subactor| {
                #(
                    each_subactor.#handler_mods(|&#msg_names(#(#msg_args),*), subactor, world| {
                        subactor.#handler_names(#(#handler_params,)* world)#maybe_fate_returns
                    });
                )*
                }));

                system.extend::<Swarm<#setup_types_3>, _>(|mut the_swarm| {
                #(
                    the_swarm.#swarm_handler_mods(
                        |&#swarm_msg_names(#(#swarm_msg_args),*), _, world| {
                            #types_for_swarm_handlers::#swarm_handler_names(
                                #(#swarm_handler_params,)* world
                            )#swarm_maybe_fate_returns
                    });
                )*

                #(
                    the_swarm.#init_handler_mods(
                        |&#init_msg_names(id, #(#init_msg_args),*), swarm, world| {
                        let mut subactor = #types_for_init_handlers_2::#init_handler_names(
                            id, #(#init_handler_params,)* world
                        );
                        unsafe {swarm.add_manually_with_id(&mut subactor, id._raw_id)};
                        ::std::mem::forget(subactor);
                        Fate::Live
                    });
                )*
                });
            )*
        )
    }

    pub fn generate_trait_ids_and_messages(&self) -> Tokens {
        let trait_ids_1: Vec<_> = self.traits.keys().map(trait_name_to_id).collect();
        let trait_ids_2 = trait_ids_1.clone();
        let (handler_names, swarm_handler_names, _) =
            self.map_trait_handlers(|_, handler| handler.name.clone());
        let (handler_args, swarm_handler_args, _) =
            self.map_trait_handlers_args(arg_as_ident_and_type);
        let (msg_names_1, swarm_msg_names_1, _) = self.map_trait_handlers(|trait_name, handler| {
            let msg_prefix = trait_to_message_prefix(&trait_name.segments.last().unwrap().ident);
            Ident::new(format!("{}_{}", msg_prefix, handler.name))
        });
        let (msg_names_2, swarm_msg_names_2) = (msg_names_1.clone(), swarm_msg_names_1.clone());
        let (msg_params, swarm_msg_params, _) = self.map_trait_handlers_args(arg_as_value);
        let (msg_param_types, swarm_msg_param_types, _) =
            self.map_trait_handlers_args(arg_as_value_type);
        let (msg_derives, swarm_msg_derives, _) =
            self.map_trait_handlers(|_, handler| if handler.arguments.is_empty() {
                quote!(#[derive(Copy, Clone)])
            } else {
                quote!(#[derive(Compact, Clone)])
            });

        quote!(
            #(
            #[derive(Copy, Clone, PartialEq, Eq, Hash)]
            pub struct #trait_ids_1 {
                pub _raw_id: ID
            }

            impl #trait_ids_2 {
                #(
                pub fn #handler_names(&self #(,#handler_args)*, world: &mut World) {
                    world.send(self._raw_id, #msg_names_1(#(#msg_params),*));
                }
                )*

                #(
                pub fn #swarm_handler_names(#(#swarm_handler_args,)* world: &mut World) {
                    let swarm = world.local_broadcast::<Swarm<Self>>();
                    world.send(swarm, #swarm_msg_names_1(#(#swarm_msg_params),*));
                }
                )*
            }

            #(
            #[allow(non_camel_case_types)]
            #msg_derives
            pub struct #msg_names_2(#(pub #msg_param_types),*);
            )*

            #(
            #[allow(non_camel_case_types)]
            #swarm_msg_derives
            pub struct #swarm_msg_names_2(#(pub #swarm_msg_param_types),*);
            )*
            )*
        )
    }

    pub fn generate_actor_ids_messages_and_conversions(&self) -> Tokens {
        let actor_here_names_1: Vec<_> = self.actors
            .iter()
            .filter_map(|(actor_name, actor_def)| if actor_def.defined_here {
                Some(actor_name)
            } else {
                None
            })
            .collect();
        let (actor_here_names_2, actor_here_names_3, actor_here_names_4) =
            (
                actor_here_names_1.clone(),
                actor_here_names_1.clone(),
                actor_here_names_1.clone(),
            );
        let actor_here_ids_1: Vec<_> = self.actors
            .iter()
            .filter_map(|(actor_name, actor_def)| if actor_def.defined_here {
                Some(actor_name_to_id(actor_name))
            } else {
                None
            })
            .collect();
        let (actor_here_ids_2, actor_here_ids_3, actor_here_ids_4, actor_here_ids_5) = (
            actor_here_ids_1.clone(),
            actor_here_ids_1.clone(),
            actor_here_ids_1.clone(),
            actor_here_ids_1.clone(),
        );

        let actor_ids: Vec<_> = self.actors.keys().map(actor_name_to_id).collect();
        let (handler_names, swarm_handler_names, init_handler_names) =
            self.map_handlers(OnlyOwn, |_, handler| handler.name.clone());
        let (handler_args, swarm_handler_args, init_handler_args) =
            self.map_handlers_args(OnlyOwn, arg_as_ident_and_type);
        let (msg_names_1, swarm_msg_names_1, init_msg_names_1) =
            self.map_handlers(OnlyOwn, |actor_name, handler| {
                let msg_prefix = typ_to_message_prefix(actor_name, None);
                Ident::new(format!("{}_{}", msg_prefix, handler.name))
            });
        let (msg_names_2, swarm_msg_names_2, init_msg_names_2) = (
            msg_names_1.clone(),
            swarm_msg_names_1.clone(),
            init_msg_names_1.clone(),
        );
        let (msg_params, swarm_msg_params, init_msg_params) =
            self.map_handlers_args(OnlyOwn, arg_as_value);
        let (msg_param_types, swarm_msg_param_types, init_msg_param_types) =
            self.map_handlers_args(OnlyOwn, arg_as_value_type);
        let (msg_derives, swarm_msg_derives, init_msg_derives) =
            self.map_handlers(OnlyOwn, |_, handler| if handler.arguments.is_empty() {
                quote!(#[derive(Copy, Clone)])
            } else {
                quote!(#[derive(Compact, Clone)])
            });
        let (_, actor_types_for_swarm_handlers, actor_types_for_init_handlers_1) =
            self.map_handlers(OnlyOwn, |actor_name, _| actor_name.clone());
        let actor_types_for_init_handlers_2 = actor_types_for_init_handlers_1.clone();

        let (_, _, actor_ids_for_init_handlers) =
            self.map_handlers(OnlyOwn, |actor_name, _| actor_name_to_id(actor_name));
        let actor_ids_for_init_msgs = actor_ids_for_init_handlers.clone();

        let actor_trait_ids_1: Vec<Vec<_>> = self.actors
            .iter()
            .map(|(_, actor_def)| {
                actor_def.impls.iter().map(trait_name_to_id).collect()
            })
            .collect();
        let actor_trait_ids_2 = actor_trait_ids_1.clone();
        let actor_ids_for_traits: Vec<Vec<_>> = self.actors
            .iter()
            .map(|(actor_name, actor_def)| {
                actor_def
                    .impls
                    .iter()
                    .map(|_| actor_name_to_id(actor_name))
                    .collect()
            })
            .collect();

        quote!(
            #(
            impl SubActor for #actor_here_names_1 {
                fn id(&self) -> ID {
                    self.id._raw_id
                }
                unsafe fn set_id(&mut self, id: ID) {
                    self.id._raw_id = id;
                }
            }

            #[derive(Copy, Clone, PartialEq, Eq, Hash)]
            pub struct #actor_here_ids_1 {
                pub _raw_id: ID
            }

            impl #actor_here_ids_2 {
                pub fn local_first(world: &mut World) -> Self {
                    #actor_here_ids_3 {
                        _raw_id: world.local_first::<Swarm<#actor_here_names_2>>()
                    }
                }

                pub fn local_broadcast(world: &mut World) -> Self {
                    #actor_here_ids_4 {
                        _raw_id: world.local_broadcast::<Swarm<#actor_here_names_3>>()
                    }
                }

                pub fn global_broadcast(world: &mut World) -> Self {
                    #actor_here_ids_5 {
                        _raw_id: world.global_broadcast::<Swarm<#actor_here_names_4>>()
                    }
                }
            }
            )*

            #(
            impl #actor_ids {
                #(
                pub fn #handler_names(&self #(,#handler_args)*, world: &mut World) {
                    world.send(self._raw_id, #msg_names_1(#(#msg_params),*));
                }
                )*

                #(
                pub fn #swarm_handler_names(#(#swarm_handler_args,)* world: &mut World) {
                    let swarm = world.local_broadcast::<Swarm<#actor_types_for_swarm_handlers>>();
                    world.send(swarm, #swarm_msg_names_1(#(#swarm_msg_params),*));
                }
                )*

                #(
                pub fn #init_handler_names(#(#init_handler_args,)* world: &mut World) -> Self {
                    let id = #actor_ids_for_init_handlers {
                        _raw_id: world.allocate_subactor_id::<#actor_types_for_init_handlers_1>()
                    };
                    let swarm = world.local_broadcast::<Swarm<#actor_types_for_init_handlers_2>>();
                    world.send(swarm, #init_msg_names_1(id, #(#init_msg_params),*));
                    id
                }
                )*
            }

            #(
            #[allow(non_camel_case_types)]
            #msg_derives
            pub struct #msg_names_2(#(pub #msg_param_types),*);
            )*

            #(
            #[allow(non_camel_case_types)]
            #swarm_msg_derives
            pub struct #swarm_msg_names_2(#(pub #swarm_msg_param_types),*);
            )*

            #(
            #[allow(non_camel_case_types)]
            #init_msg_derives
            pub struct #init_msg_names_2(
                pub #actor_ids_for_init_msgs, #(pub #init_msg_param_types),*
            );
            )*
            )*

            #(
                #(
                impl Into<#actor_trait_ids_1> for #actor_ids_for_traits {
                    fn into(self) -> #actor_trait_ids_2 {
                        unsafe {
                            ::std::mem::transmute(self)
                        }
                    }
                }
                )*
            )*
        )
    }
}

fn actor_name_to_id(actor_name: &Ty) -> Ident {
    let segments = match *actor_name {
        Ty::Path(_, ref path) => path.segments.clone(),
        _ => unimplemented!(),
    };
    Ident::new(format!("{}ID", segments.last().unwrap().ident))
}

fn trait_name_to_id(trait_name: &TraitName) -> Ident {
    Ident::new(format!("{}ID", trait_name.segments.last().unwrap().ident))
}

fn arg_as_ident_and_type(arg: &FnArg) -> FnArg {
    match arg {
        &FnArg::Captured(ref ident, Ty::Rptr(_, ref ty_box)) => {
            FnArg::Captured(ident.clone(), ty_box.ty.clone())
        }
        other => other.clone(),
    }
}

fn arg_as_value(arg: &FnArg) -> Ident {
    match arg {
        &FnArg::Captured(Pat::Ident(_, ref ident, _), _) => ident.clone(),
        _ => unimplemented!(),
    }
}

fn arg_as_value_type(arg: &FnArg) -> Ty {
    match arg {
        &FnArg::Captured(_, Ty::Rptr(_, ref ty_box)) => ty_box.ty.clone(),
        &FnArg::Captured(_, ref other) => other.clone(),
        _ => unimplemented!(),
    }
}

fn typ_to_message_prefix(typ: &Ty, with_trait: Option<&Path>) -> String {
    let segments = if let Some(path) = with_trait {
        &path.segments
    } else if let &Ty::Path(_, Path { ref segments, .. }) = typ {
        segments
    } else {
        unimplemented!()
    };

    let prefixed = segments
        .iter()
        .map(|s| s.ident.as_ref())
        .collect::<Vec<_>>()
        .join("_");
    format!("MSG_{}", prefixed)
}

fn trait_to_message_prefix(ident: &Ident) -> String {
    format!("MSG_{}", ident)
}
