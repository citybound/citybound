#![recursion_limit="128"]
#![feature(conservative_impl_trait)]
extern crate syn;
#[macro_use]
extern crate quote;
use syn::*;
use quote::Tokens;
extern crate glob;
use glob::glob;

use std::fs::{File, metadata};
use std::io::Read;
use std::io::Write;
use std::process::Command;

extern crate ordermap;
use ordermap::OrderMap;

use BindingMode::{ByRef, ByValue};
use Mutability::Immutable;

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
    actors: OrderMap<ActorName, ActorDef>,
    traits: OrderMap<TraitName, TraitDef>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum WhichHandlers {
    OnlyOwn,
    AlsoFromTraits,
}
use WhichHandlers::*;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum WhichActors {
    OnlyActors,
    OnlySubActors,
    All,
}
use WhichActors::*;

impl Model {
    pub fn get(&self, act: WhichActors) -> Vec<(&ActorName, &ActorDef)> {
        self.actors
            .iter()
            .filter(|&(_, actor_def)| match act {
                OnlyActors => !actor_def.is_subactor,
                OnlySubActors => actor_def.is_subactor,
                All => true,
            })
            .collect()
    }

    pub fn map_handlers<O, F>(
        &self,
        han: WhichHandlers,
        act: WhichActors,
        mapping_f: F,
    ) -> Vec<Vec<O>>
    where
        F: Fn(&ActorName, &Handler) -> O,
    {
        self.get(act)
            .into_iter()
            .map(|(actor_name, actor_def)| {
                actor_def
                    .handlers
                    .iter()
                    .filter(|&handler| {
                        han == AlsoFromTraits || handler.from_trait.is_none()
                    })
                    .map(|handler| mapping_f(actor_name, handler))
                    .collect()
            })
            .collect()
    }

    pub fn map_handlers_args<O, F>(
        &self,
        han: WhichHandlers,
        act: WhichActors,
        mapping_f: F,
    ) -> Vec<Vec<Vec<O>>>
    where
        F: Fn(&FnArg) -> O,
    {
        self.map_handlers(han, act, |_, handler| {
            handler.arguments.iter().map(&mapping_f).collect()
        })
    }

    pub fn map_trait_handlers<O, F>(&self, mapping_f: F) -> Vec<Vec<O>>
    where
        F: Fn(&TraitName, &Handler) -> O,
    {
        self.traits
            .iter()
            .map(|(trait_name, trait_def)| {
                trait_def
                    .handlers
                    .iter()
                    .map(|handler| mapping_f(trait_name, handler))
                    .collect()
            })
            .collect()
    }

    pub fn map_trait_handlers_args<O, F>(&self, mapping_f: F) -> Vec<Vec<Vec<O>>>
    where
        F: Fn(&FnArg) -> O,
    {
        self.map_trait_handlers(|_, handler| {
            handler.arguments.iter().map(&mapping_f).collect()
        })
    }

    pub fn generate_setups(&self, act: WhichActors) -> Tokens {
        let setup_types_1: Vec<_> = self.get(act).into_iter().map(|(ty, _)| ty).collect();
        let setup_types_2 = setup_types_1.clone();
        let handler_mods: Vec<Vec<_>> = self.map_handlers(AlsoFromTraits, act, |_, handler| {
            Ident::from(if handler.critical {
                "on_critical"
            } else {
                "on"
            })
        });
        let msg_names: Vec<Vec<_>> =
            self.map_handlers(AlsoFromTraits, act, |actor_name, handler| {
                let msg_prefix = typ_to_message_prefix(actor_name, handler.from_trait.as_ref());
                Ident::new(format!("{}_{}", msg_prefix, handler.name))
            });
        let msg_args: Vec<Vec<Vec<_>>> =
            self.map_handlers_args(AlsoFromTraits, act, |arg| match arg {
                &FnArg::Captured(Pat::Ident(_, ref ident, _), ref ty) => {
                    match ty {
                        &Ty::Rptr(_, _) => Pat::Ident(ByRef(Immutable), ident.clone(), None),
                        _ => Pat::Ident(ByValue(Immutable), ident.clone(), None),
                    }
                }
                _ => unimplemented!(),
            });
        let handler_names =
            self.map_handlers(AlsoFromTraits, act, |_, handler| handler.name.clone());
        let handler_params: Vec<Vec<Vec<_>>> =
            self.map_handlers_args(AlsoFromTraits, act, |arg| match arg {
                &FnArg::Captured(Pat::Ident(_, ref ident, _), _) => ident.clone(),
                _ => unimplemented!(),
            });
        let maybe_fate_returns: Vec<Vec<_>> =
            self.map_handlers(AlsoFromTraits, act, |_, handler| if handler.returns_fate {
                quote!()
            } else {
                quote!(; Fate::Live)
            });

        if act == OnlyActors {
            quote!(
                #(
                    system.extend::<#setup_types_1, _>(|mut definer| {
                    #(
                        definer.#handler_mods(|&#msg_names(#(#msg_args),*), actor, world| {
                            actor.#handler_names(#(#handler_params,)* world)#maybe_fate_returns
                        });
                    )*
                    });
                )*
            )
        } else {
            quote!(
                #(
                    system.extend::<Swarm<#setup_types_1>, _>(Swarm::<#setup_types_2>::subactors(|mut definer| {
                    #(
                        definer.#handler_mods(|&#msg_names(#(#msg_args),*), actor, world| {
                            actor.#handler_names(#(#handler_params),*, world)#maybe_fate_returns
                        });
                    )*
                    }));
                )*
            )
        }

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
        let handler_names = self.map_handlers(OnlyOwn, All, |_, handler| handler.name.clone());
        let handler_args: Vec<Vec<Vec<_>>> =
            self.map_handlers_args(OnlyOwn, All, arg_as_ident_and_type);
        let msg_names_1: Vec<Vec<_>> = self.map_handlers(OnlyOwn, All, |actor_name, handler| {
            let msg_prefix = typ_to_message_prefix(actor_name, None);
            Ident::new(format!("{}_{}", msg_prefix, handler.name))
        });
        let msg_names_2 = msg_names_1.clone();
        let msg_params: Vec<Vec<Vec<_>>> = self.map_handlers_args(OnlyOwn, All, arg_as_value);
        let msg_param_types: Vec<Vec<Vec<_>>> =
            self.map_handlers_args(OnlyOwn, All, arg_as_value_type);
        let msg_derives: Vec<Vec<_>> =
            self.map_handlers(OnlyOwn, All, |_, handler| if handler.arguments.is_empty() {
                quote!(#[derive(Copy, Clone)])
            } else {
                quote!(#[derive(Compact, Clone)])
            });

        let actor_trait_ids_1: Vec<Vec<_>> = self.get(All)
            .into_iter()
            .map(|(_, actor_def)| {
                actor_def.impls.iter().map(trait_name_to_id).collect()
            })
            .collect();
        let actor_trait_ids_2 = actor_trait_ids_1.clone();
        let actor_ids_for_traits: Vec<Vec<_>> = self.get(All)
            .into_iter()
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
                    #actor_here_ids_3 { _raw_id: world.id::<#actor_here_names>() }
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

#[derive(Default)]
pub struct ActorDef {
    handlers: Vec<Handler>,
    impls: Vec<TraitName>,
    is_subactor: bool,
    defined_here: bool,
}

#[derive(Default)]
pub struct TraitDef {
    handlers: Vec<Handler>,
}

#[derive(Clone)]
pub struct Handler {
    name: Ident,
    arguments: Vec<FnArg>,
    critical: bool,
    returns_fate: bool,
    from_trait: Option<TraitName>,
}


pub fn parse(file: &str) -> Model {
    let mut model = Model::default();

    for item in parse_crate(file).unwrap().items.iter() {
        let ident = &item.ident;
        let attrs = &item.attrs;
        match item.node {
            ItemKind::Struct(_, _) => {
                let ident_as_seg: PathSegment = ident.clone().into();
                let actor_def = model
                    .actors
                    .entry(Ty::Path(None, syn::Path::from(ident_as_seg)))
                    .or_insert_with(Default::default);
                actor_def.defined_here = true;
            }
            ItemKind::Impl(_, _, _, ref maybe_trait, ref actor_name, ref impl_items) => {
                let actor_def = model.actors.entry((**actor_name).clone()).or_insert_with(
                    Default::default,
                );
                actor_def.is_subactor = attrs.iter().any(|attr| {
                    attr.is_sugared_doc &&
                        attr.value == MetaItem::NameValue("doc".into(), "/// Subactor".into())
                });
                if let Some(ref trait_name) = *maybe_trait {
                    actor_def.impls.push(trait_name.clone());
                    actor_def.handlers.extend(handlers_from_impl_items(
                        impl_items,
                        Some(trait_name.clone()),
                    ));
                } else {
                    actor_def.handlers.extend(
                        handlers_from_impl_items(impl_items, None),
                    );
                }
            }
            ItemKind::Trait(_, _, _, ref trait_items) => {
                let trait_name: TraitName = syn::Path::from(PathSegment::from(ident.clone()));
                let trait_def = model.traits.entry(trait_name.clone()).or_insert_with(
                    Default::default,
                );
                trait_def.handlers.extend(
                    handlers_from_trait_items(trait_items),
                );
            }
            _ => {}
        }
    }

    model.actors.retain(|ref _name, ref actor_def| {
        !actor_def.handlers.is_empty()
    });

    model
}

pub fn generate(model: &Model) -> String {
    let traits_msgs = model.generate_trait_ids_and_messages();
    let actors_msgs = model.generate_actor_ids_messages_and_conversions();
    let setup = model.generate_setups(OnlyActors);
    let sub_setup = model.generate_setups(OnlySubActors);

    quote!(
        //! This is all auto-generated. Do not touch.
        use kay::ActorSystem;
        use kay::swarm::Swarm;
        use super::*;

        #traits_msgs
        #actors_msgs
        pub fn auto_setup(system: &mut ActorSystem) {
            #setup
            #sub_setup
        }

    ).into_string()
}

fn handlers_from_impl_items(
    impl_items: &[ImplItem],
    with_trait: Option<TraitName>,
) -> Vec<Handler> {
    impl_items
        .iter()
        .filter_map(|impl_item| if let &ImplItem {
            ident: ref fn_name,
            ref vis,
            node: ImplItemKind::Method(ref sig, _),
            ref attrs,
            ..
        } = impl_item
        {
            if with_trait.is_some() || *vis == Visibility::Public {
                handler_from(fn_name, sig, attrs, with_trait.clone())
            } else {
                None
            }
        } else {
            None
        })
        .collect()
}

fn handlers_from_trait_items(trait_items: &[TraitItem]) -> Vec<Handler> {
    trait_items
        .iter()
        .filter_map(|trait_item| if let &TraitItem {
            ident: ref fn_name,
            node: TraitItemKind::Method(ref sig, _),
            ..
        } = trait_item
        {
            handler_from(fn_name, sig, &[], None)
        } else {
            None
        })
        .collect()
}

fn handler_from(
    fn_name: &Ident,
    sig: &MethodSig,
    attrs: &[Attribute],
    from_trait: Option<TraitName>,
) -> Option<Handler> {
    check_handler(sig).map(|args| {
        let returns_fate = match sig.decl.output {
            FunctionRetTy::Default => false,
            FunctionRetTy::Ty(Ty::Path(_, Path { ref segments, .. })) => {
                segments.iter().any(|s| s.ident.as_ref() == "Fate")
            }
            _ => unimplemented!(),
        };

        let is_critical = attrs.iter().any(|attr| {
            attr.is_sugared_doc &&
                attr.value == MetaItem::NameValue("doc".into(), "/// Critical".into())
        });

        Handler {
            name: fn_name.clone(),
            arguments: args.to_vec(),
            critical: is_critical,
            returns_fate: returns_fate,
            from_trait: from_trait.clone(),
        }
    })
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

pub fn check_handler(sig: &MethodSig) -> Option<&[FnArg]> {
    if let Some(&FnArg::SelfRef(_, Mutability::Mutable)) = sig.decl.inputs.get(0) {
        if let Some(&FnArg::Captured(_, Ty::Rptr(_, ref ty_box))) = sig.decl.inputs.last() {
            if let &MutTy {
                mutability: Mutability::Mutable,
                ty: Ty::Path(_, ref path),
            } = &**ty_box
            {
                if path.segments.last().unwrap().ident == Ident::new("World") {
                    let args = &sig.decl.inputs[1..(sig.decl.inputs.len() - 1)];
                    Some(args)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}
