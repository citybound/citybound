use syn::*;
use quote::Tokens;
use {Model, ActorName, TraitName, Handler};
use BindingMode::{ByRef, ByValue};
use Mutability::Immutable;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum WhichHandlers {
    OnlyOwn,
    AlsoFromTraits,
}
use self::WhichHandlers::*;

impl Model {
    pub fn map_handlers<O, F>(&self, han: WhichHandlers, map_f: F) -> Vec<Vec<O>>
    where
        F: Fn(&ActorName, &Handler) -> O,
    {
        self.actors
            .iter()
            .map(|(actor_name, actor_def)| {
                actor_def
                    .handlers
                    .iter()
                    .filter(|&handler| {
                        han == AlsoFromTraits || handler.from_trait.is_none()
                    })
                    .map(|handler| map_f(actor_name, handler))
                    .collect()
            })
            .collect()
    }

    pub fn map_handlers_args<O, F>(&self, han: WhichHandlers, map_f: F) -> Vec<Vec<Vec<O>>>
    where
        F: Fn(&FnArg) -> O,
    {
        self.map_handlers(han, |_, handler| {
            handler.arguments.iter().map(&map_f).collect()
        })
    }

    pub fn map_trait_handlers<O, F>(&self, map_f: F) -> Vec<Vec<O>>
    where
        F: Fn(&TraitName, &Handler) -> O,
    {
        self.traits
            .iter()
            .map(|(name, def)| {
                def.handlers
                    .iter()
                    .map(|handler| map_f(name, handler))
                    .collect()
            })
            .collect()
    }

    pub fn map_trait_handlers_args<O, F>(&self, map_f: F) -> Vec<Vec<Vec<O>>>
    where
        F: Fn(&FnArg) -> O,
    {
        self.map_trait_handlers(|_, handler| handler.arguments.iter().map(&map_f).collect())
    }

    pub fn generate_setups(&self) -> Tokens {
        let setup_types_1: Vec<_> = self.actors.iter().map(|(ty, _)| ty).collect();
        let setup_types_2 = setup_types_1.clone();
        let handler_mods: Vec<Vec<_>> = self.map_handlers(AlsoFromTraits, |_, handler| {
            Ident::from(if handler.critical {
                "on_critical"
            } else {
                "on"
            })
        });
        let msg_names: Vec<Vec<_>> = self.map_handlers(AlsoFromTraits, |actor_name, handler| {
            let msg_prefix = typ_to_message_prefix(actor_name, handler.from_trait.as_ref());
            Ident::new(format!("{}_{}", msg_prefix, handler.name))
        });
        let msg_args: Vec<Vec<Vec<_>>> = self.map_handlers_args(AlsoFromTraits, |arg| match arg {
            &FnArg::Captured(Pat::Ident(_, ref ident, _), ref ty) => {
                match ty {
                    &Ty::Rptr(_, _) => Pat::Ident(ByRef(Immutable), ident.clone(), None),
                    _ => Pat::Ident(ByValue(Immutable), ident.clone(), None),
                }
            }
            _ => unimplemented!(),
        });
        let handler_names = self.map_handlers(AlsoFromTraits, |_, handler| handler.name.clone());
        let handler_params: Vec<Vec<Vec<_>>> =
            self.map_handlers_args(AlsoFromTraits, |arg| match arg {
                &FnArg::Captured(Pat::Ident(_, ref ident, _), _) => ident.clone(),
                _ => unimplemented!(),
            });
        let maybe_fate_returns: Vec<Vec<_>> =
            self.map_handlers(AlsoFromTraits, |_, handler| if handler.returns_fate {
                quote!()
            } else {
                quote!(; Fate::Live)
            });

        quote!(
            #(
                system.extend::<Swarm<#setup_types_1>, _>(Swarm::<#setup_types_2>::subactors(|mut definer| {
                #(
                    definer.#handler_mods(|&#msg_names(#(#msg_args),*), actor, world| {
                        actor.#handler_names(#(#handler_params,)* world)#maybe_fate_returns
                    });
                )*
                }));
            )*
        )
    }

    pub fn generate_trait_ids_and_messages(&self) -> Tokens {
        let trait_ids_1: Vec<_> = self.traits.keys().map(trait_name_to_id).collect();
        let trait_ids_2 = trait_ids_1.clone();
        let handler_names: Vec<Vec<_>> = self.map_trait_handlers(|_, handler| handler.name.clone());
        let handler_args: Vec<Vec<Vec<_>>> = self.map_trait_handlers_args(arg_as_ident_and_type);
        let msg_names_1: Vec<Vec<_>> = self.map_trait_handlers(|trait_name, handler| {
            let msg_prefix = trait_to_message_prefix(&trait_name.segments.last().unwrap().ident);
            Ident::new(format!("{}_{}", msg_prefix, handler.name))
        });
        let msg_names_2 = msg_names_1.clone();
        let msg_params: Vec<Vec<Vec<_>>> = self.map_trait_handlers_args(arg_as_value);
        let msg_param_types: Vec<Vec<Vec<_>>> = self.map_trait_handlers_args(arg_as_value_type);
        let msg_derives: Vec<Vec<_>> =
            self.map_trait_handlers(|_, handler| if handler.arguments.is_empty() {
                quote!(#[derive(Copy, Clone)])
            } else {
                quote!(#[derive(Compact, Clone)])
            });

        quote!(
            #(
            #[derive(Copy, Clone)]
            pub struct #trait_ids_1 {
                pub _raw_id: ID
            }

            impl #trait_ids_2 {
                #(
                pub fn #handler_names(&self #(,#handler_args)*, world: &mut World) {
                    world.send(self._raw_id, #msg_names_1(#(#msg_params),*));
                }
                )*
            }

            #(
            #[allow(non_camel_case_types)]
            #msg_derives
            struct #msg_names_2(#(#msg_param_types),*);
            )*
            )*
        )
    }

    pub fn generate_actor_ids_messages_and_conversions(&self) -> Tokens {
        let actor_here_names: Vec<_> = self.actors
            .iter()
            .filter_map(|(actor_name, actor_def)| if actor_def.defined_here {
                Some(actor_name)
            } else {
                None
            })
            .collect();
        let actor_here_ids_1: Vec<_> = self.actors
            .iter()
            .filter_map(|(actor_name, actor_def)| if actor_def.defined_here {
                Some(actor_name_to_id(actor_name))
            } else {
                None
            })
            .collect();
        let (actor_here_ids_2, actor_here_ids_3) =
            (actor_here_ids_1.clone(), actor_here_ids_1.clone());

        let actor_ids: Vec<_> = self.actors.keys().map(actor_name_to_id).collect();
        let handler_names = self.map_handlers(OnlyOwn, |_, handler| handler.name.clone());
        let handler_args: Vec<Vec<Vec<_>>> = self.map_handlers_args(OnlyOwn, arg_as_ident_and_type);
        let msg_names_1: Vec<Vec<_>> = self.map_handlers(OnlyOwn, |actor_name, handler| {
            let msg_prefix = typ_to_message_prefix(actor_name, None);
            Ident::new(format!("{}_{}", msg_prefix, handler.name))
        });
        let msg_names_2 = msg_names_1.clone();
        let msg_params: Vec<Vec<Vec<_>>> = self.map_handlers_args(OnlyOwn, arg_as_value);
        let msg_param_types: Vec<Vec<Vec<_>>> = self.map_handlers_args(OnlyOwn, arg_as_value_type);
        let msg_derives: Vec<Vec<_>> =
            self.map_handlers(OnlyOwn, |_, handler| if handler.arguments.is_empty() {
                quote!(#[derive(Copy, Clone)])
            } else {
                quote!(#[derive(Compact, Clone)])
            });

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
            #[derive(Copy, Clone)]
            pub struct #actor_here_ids_1 {
                pub _raw_id: ID
            }

            impl #actor_here_ids_2 {
                pub fn in_world(world: &mut World) -> Self {
                    #actor_here_ids_3 { _raw_id: world.id::<Swarm<#actor_here_names>>() }
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
            }

            #(
            #[allow(non_camel_case_types)]
            #msg_derives
            struct #msg_names_2(#(#msg_param_types),*);
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
