#![recursion_limit="128"]
extern crate syn;
#[macro_use]
extern crate quote;
use syn::*;
use quote::Tokens;

extern crate ordermap;
use ordermap::OrderMap;

pub fn generate(file: &str) -> String {

    let mut setup_map = OrderMap::<Ty, (usize, Vec<Tokens>)>::new();
    let mut id_map = OrderMap::<Ty, Vec<Tokens>>::new();
    let mut trait_id_map = OrderMap::<Ident, Vec<Tokens>>::new();
    let mut msg_map = OrderMap::<Ty, Vec<Tokens>>::new();
    let mut trait_msg_map = OrderMap::<Ident, Vec<Tokens>>::new();

    let mut next_actor_index = 0;

    for item in parse_crate(file).unwrap().items.iter() {
        match *item {
            Item { node: ItemKind::Impl(_, _, _, None, ref typ, ref impl_items), .. } => {
                if !setup_map.contains_key(typ) {
                    setup_map.insert((**typ).clone(), (next_actor_index, Vec::new()));
                    next_actor_index += 1;
                };
                setup_map
                    .get_mut(typ)
                    .unwrap()
                    .1
                    .extend(generate_setup(typ, impl_items, None));
                id_map
                    .entry((**typ).clone())
                    .or_insert_with(Vec::new)
                    .extend(generate_id(typ, impl_items, None));
                msg_map
                    .entry((**typ).clone())
                    .or_insert_with(Vec::new)
                    .push(generate_msgs(typ, impl_items));
            }
            Item {
                node: ItemKind::Impl(_, _, _, Some(ref trt), ref typ, ref impl_items), ..
            } => {
                if !setup_map.contains_key(typ) {
                    setup_map.insert((**typ).clone(), (next_actor_index, Vec::new()));
                    next_actor_index += 1;
                };
                setup_map
                    .get_mut(typ)
                    .unwrap()
                    .1
                    .extend(generate_setup(typ, impl_items, Some(trt)));
                id_map
                    .entry((**typ).clone())
                    .or_insert_with(Vec::new)
                    .extend(generate_id(typ, impl_items, Some(trt)));
                //panic!("{} for {}", quote!(#trt), quote!(#typ).into_string());
            }
            Item {
                ref ident,
                node: ItemKind::Trait(_, _, _, ref trait_items),
                ..
            } => {
                trait_id_map
                    .entry(ident.clone())
                    .or_insert_with(Vec::new)
                    .extend(generate_trait_id(ident, trait_items));
                trait_msg_map
                    .entry(ident.clone())
                    .or_insert_with(Vec::new)
                    .push(generate_trait_msgs(ident, trait_items));
            }
            _ => {}
        }
    }

    let (setup_types, setup_idxs, setup_defs) =
        (setup_map.keys(),
         setup_map.values().map(|&(idx, _)| Ident::new(idx)),
         setup_map.values().map(|&(_, ref defs)| defs));
    let (types, id_defs) = (id_map.keys().collect::<Vec<_>>(), id_map.values());
    let msg_defs = msg_map.values().flat_map(|v| v);
    let id_types = types
        .iter()
        .map(|typ| match typ {
                 &&Ty::Path(None, Path { global, ref segments }) => {
                     let mut new_segments = segments.clone();
                     new_segments.last_mut().unwrap().ident =
                         Ident::new(new_segments.last_mut().unwrap().ident.as_ref().to_owned() +
                                    "ID");
                     Ty::Path(None, Path { global, segments: new_segments })
                 }
                 _ => unimplemented!(),
             })
        .collect::<Vec<_>>();
    let types_1 = types.iter();
    let id_types_1 = id_types.iter();
    let id_types_2 = id_types.iter();
    let id_types_3 = id_types.iter();
    let id_types_4 = id_types.iter();

    let (trait_types, trait_id_defs) = (trait_id_map.keys().collect::<Vec<_>>(),
                                        trait_id_map.values());
    let trait_msg_defs = trait_msg_map.values().flat_map(|v| v);
    let trait_id_types = trait_types
        .iter()
        .map(|ident| {
            Ty::Path(None,
                     Path {
                         global: false,
                         segments: vec![Ident::new(format!("{}ID", ident)).into()],
                     })
        })
        .collect::<Vec<_>>();
    let trait_types_1 = types.iter();
    let trait_id_types_1 = trait_id_types.iter();
    let trait_id_types_2 = trait_id_types.iter();
    let trait_id_types_3 = trait_id_types.iter();
    let trait_id_types_4 = trait_id_types.iter();

    quote!(
        //! This is all auto-generated. Do not touch.
        use kay::ActorSystem;
        use super::*;

        pub fn auto_setup(system: &mut ActorSystem, initial: (#(#setup_types),*,)) {
            #(
                system.add(initial.#setup_idxs, |mut definer| {
                    #(#setup_defs)*
                });
            )*
        }

        #(#msg_defs)*

        #(
            #[derive(Copy, Clone)]
            pub struct #id_types_1 {
                raw_id: ID,
            }

            impl #id_types_2 {
                pub fn in_world(world: &mut World) -> Self {
                    #id_types_3 {raw_id: world.id::<#types_1>()}
                }
            }

            impl #id_types_4 {
                #(#id_defs)*
            }
        )*

        #(#trait_msg_defs)*

        #(
            #[derive(Copy, Clone)]
            pub struct #trait_id_types_1 {
                raw_id: ID,
            }

            impl #trait_id_types_2 {
                pub fn in_world(world: &mut World) -> Self {
                    #trait_id_types_3 {raw_id: world.id::<#trait_types_1>()}
                }
            }

            impl #trait_id_types_4 {
                #(#trait_id_defs)*
            }
        )*
    ).into_string()
}

pub fn generate_setup(typ: &Ty, impl_items: &[ImplItem], with_trait: Option<&Path>) -> Vec<Tokens> {
    let msg_prefix = typ_to_message_prefix(typ, with_trait);
    let setup_calls = impl_items.iter().filter_map(|impl_item| if let &ImplItem {
               ident: ref fn_name,
               ref vis,
               node: ImplItemKind::Method(ref sig, _),
               ref attrs,
               ..
           } = impl_item {
        if with_trait.is_some() || *vis == Visibility::Public {
            check_handler(sig).map(|args| {
                let reffed_args = args.iter()
                    .map(|arg| match arg {
                             &FnArg::Captured(Pat::Ident(_, ref ident, _), ref ty) => {
                                 match ty {
                                     &Ty::Rptr(_, _) => {
                                         Pat::Ident(BindingMode::ByRef(Mutability::Immutable),
                                                    ident.clone(),
                                                    None)
                                     }
                                     _ => {
                                         Pat::Ident(BindingMode::ByValue(Mutability::Immutable),
                                                    ident.clone(),
                                                    None)
                                     }

                                 }
                             }
                             _ => unimplemented!(),
                         })
                    .collect::<Vec<_>>();
                let params = args.iter()
                    .map(|arg| match arg {
                             &FnArg::Captured(Pat::Ident(_, ref ident, _), _) => ident.clone(),
                             _ => unimplemented!(),
                         })
                    .collect::<Vec<_>>();
                let msg_name = Ident::new(format!("{}_{}", msg_prefix, fn_name));
                let returns_fate = match sig.decl.output {
                    FunctionRetTy::Default => false,
                    FunctionRetTy::Ty(Ty::Path(_, Path { ref segments, .. })) => {
                        segments.iter().any(|s| s.ident.as_ref() == "Fate")
                    }
                    _ => unimplemented!(),
                };
                let inner = if returns_fate {
                    quote!(
                        actor.#fn_name(#(#params),*, world)
                    )
                } else {
                    quote!(
                        actor.#fn_name(#(#params),*, world);
                        Fate::Live
                    )
                };
                let is_critical = attrs.iter().any(|attr| {
                    attr.is_sugared_doc &&
                    attr.value == MetaItem::NameValue("doc".into(), "/// Critical".into())
                });
                if is_critical {
                    quote!(
                        definer.on_critical(|&#msg_name(#(#reffed_args),*), actor, world| {
                            #inner
                        });
                    )
                } else {
                    quote!(
                        definer.on(|&#msg_name(#(#reffed_args),*), actor, world| {
                            #inner
                        });
                    )
                }
            })
        } else {
            None
        }
    } else {
        None
    });

    setup_calls.collect()
}

pub fn generate_id(typ: &Ty, impl_items: &[ImplItem], with_trait: Option<&Path>) -> Vec<Tokens> {
    let msg_prefix = typ_to_message_prefix(typ, with_trait);
    let id_methods =
        impl_items.iter().filter_map(|impl_item| if let &ImplItem {
                                                ident: ref fn_name,
                                                ref vis,
                                                node: ImplItemKind::Method(ref sig, _),
                                                ..
                                            } = impl_item {
                                         if with_trait.is_some() || *vis == Visibility::Public {
                                             generate_id_inner(fn_name, sig, &msg_prefix)
                                         } else {
                                             None
                                         }
                                     } else {
                                         None
                                     });

    id_methods.collect()
}

pub fn generate_trait_id(trait_ident: &Ident, trait_items: &[TraitItem]) -> Vec<Tokens> {
    let msg_prefix = trait_to_message_prefix(&trait_ident);
    let id_methods = trait_items
        .iter()
        .filter_map(|trait_item| if let &TraitItem {
                               ident: ref fn_name,
                               node: TraitItemKind::Method(ref sig, _),
                               ..
                           } = trait_item {
                        generate_id_inner(fn_name, sig, &msg_prefix)
                    } else {
                        None
                    });

    id_methods.collect()
}

pub fn generate_id_inner(fn_name: &Ident, sig: &MethodSig, msg_prefix: &str) -> Option<Tokens> {
    check_handler(sig).map(|args| {
        let owned_sig = args.iter().map(|arg| match arg {
                                            &FnArg::Captured(ref ident,
                                                             Ty::Rptr(_, ref ty_box)) => {
                                                FnArg::Captured(ident.clone(), ty_box.ty.clone())
                                            }
                                            other => other.clone(),
                                        });
        let params =
            args.iter().map(|arg| match arg {
                                &FnArg::Captured(Pat::Ident(_, ref ident, _), _) => ident.clone(),
                                _ => unimplemented!(),
                            });
        let msg_name = Ident::new(format!("{}_{}", msg_prefix, fn_name));
        quote!(
                    pub fn #fn_name(&self, #(#owned_sig),*, world: &mut World) {
                        world.send(self.raw_id, #msg_name(#(#params),*))
                    }
                )
    })
}

fn generate_msgs(typ: &Ty, impl_items: &[ImplItem]) -> Tokens {
    let msg_prefix = typ_to_message_prefix(typ, None);
    let msg_definitions = impl_items.iter().filter_map(|impl_item| if let &ImplItem {
               ident: ref fn_name,
               vis: Visibility::Public,
               node: ImplItemKind::Method(ref sig, _),
               ..
           } = impl_item {
        generate_trait_inner(fn_name, sig, &msg_prefix)
    } else {
        None
    });

    quote!(
            #(#msg_definitions)*
    )
}

fn generate_trait_msgs(trait_ident: &Ident, trait_items: &[TraitItem]) -> Tokens {
    let msg_prefix = trait_to_message_prefix(&trait_ident);
    let msg_definitions = trait_items
        .iter()
        .filter_map(|trait_item| if let &TraitItem {
                               ident: ref fn_name,
                               node: TraitItemKind::Method(ref sig, _),
                               ..
                           } = trait_item {
                        generate_trait_inner(fn_name, sig, &msg_prefix)
                    } else {
                        None
                    });

    quote!(
            #(#msg_definitions)*
    )
}

fn generate_trait_inner(fn_name: &Ident, sig: &MethodSig, msg_prefix: &str) -> Option<Tokens> {
    check_handler(sig).map(|args| {
        let field_types =
            args.iter().map(|arg| match arg {
                                &FnArg::Captured(_, Ty::Rptr(_, ref ty_box)) => &ty_box.ty,
                                &FnArg::Captured(_, ref other) => other,
                                _ => unimplemented!(),
                            });
        let msg_name = Ident::new(format!("{}_{}", msg_prefix, fn_name));
        quote!(
                    #[allow(non_camel_case_types)]
                    #[derive(Compact, Clone)]
                    struct #msg_name(#(#field_types),*);
                )
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
                   } = &**ty_box {
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
