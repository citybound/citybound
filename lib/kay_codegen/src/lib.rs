#![recursion_limit="128"]
extern crate syn;
#[macro_use]
extern crate quote;
use syn::*;
use quote::Tokens;

use std::collections::HashMap;

pub fn generate(file: &str) -> String {

    let mut setup_map = HashMap::<Ty, (usize, Vec<Tokens>)>::new();
    let mut id_map = HashMap::<Ty, Vec<Tokens>>::new();
    let mut msg_map = HashMap::<Ty, Vec<Tokens>>::new();

    let mut next_actor_index = 0;

    for item in parse_crate(file).unwrap().items.iter() {
        if let ItemKind::Impl(_, _, _, None, ref typ, ref impl_items) = item.node {
            if !setup_map.contains_key(typ) {
                setup_map.insert((**typ).clone(), (next_actor_index, Vec::new()));
                next_actor_index += 1;
            };
            setup_map
                .get_mut(typ)
                .unwrap()
                .1
                .extend(generate_setup(typ, impl_items));
            id_map
                .entry((**typ).clone())
                .or_insert_with(Vec::new)
                .extend(generate_id(typ, impl_items));
            msg_map
                .entry((**typ).clone())
                .or_insert_with(Vec::new)
                .push(generate_msgs(typ, impl_items));
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

    quote!(
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
    ).into_string()
}

pub fn generate_setup(typ: &Ty, impl_items: &[ImplItem]) -> Vec<Tokens> {
    let setup_calls = impl_items.iter().filter_map(|impl_item| if let &ImplItem {
               ident: ref fn_name,
               vis: Visibility::Public,
               node: ImplItemKind::Method(ref sig, _),
               ref attrs,
               ..
           } = impl_item {
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
            let msg_name = message_name(typ, fn_name);
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
    });

    setup_calls.collect()
}

pub fn generate_id(typ: &Ty, impl_items: &[ImplItem]) -> Vec<Tokens> {
    let id_methods = impl_items.iter().filter_map(|impl_item| if let &ImplItem {
               ident: ref fn_name,
               vis: Visibility::Public,
               node: ImplItemKind::Method(ref sig, _),
               ..
           } = impl_item {
        check_handler(sig).map(|args| {
            let owned_sig =
                args.iter().map(|arg| match arg {
                                    &FnArg::Captured(ref ident, Ty::Rptr(_, ref ty_box)) => {
                                        FnArg::Captured(ident.clone(), ty_box.ty.clone())
                                    }
                                    other => other.clone(),
                                });
            let params = args.iter().map(|arg| match arg {
                                             &FnArg::Captured(Pat::Ident(_, ref ident, _), _) => {
                                                 ident.clone()
                                             }
                                             _ => unimplemented!(),
                                         });
            let msg_name = message_name(typ, fn_name);
            quote!(
                    pub fn #fn_name(&self, #(#owned_sig),*, world: &mut World) {
                        world.send(self.raw_id, #msg_name(#(#params),*))
                    }
                )
        })
    } else {
        None
    });

    id_methods.collect()
}

pub fn generate_msgs(typ: &Ty, impl_items: &[ImplItem]) -> Tokens {
    let msg_definitions = impl_items.iter().filter_map(|impl_item| if let &ImplItem {
               ident: ref fn_name,
               vis: Visibility::Public,
               node: ImplItemKind::Method(ref sig, _),
               ..
           } = impl_item {
        check_handler(sig).map(|args| {
            let field_types =
                args.iter().map(|arg| match arg {
                                    &FnArg::Captured(_, Ty::Rptr(_, ref ty_box)) => &ty_box.ty,
                                    &FnArg::Captured(_, ref other) => other,
                                    _ => unimplemented!(),
                                });
            let msg_name = message_name(typ, fn_name);
            quote!(
                    #[allow(non_camel_case_types)]
                    #[derive(Compact, Clone)]
                    struct #msg_name(#(#field_types),*);
                )
        })
    } else {
        None
    });

    quote!(
            #(#msg_definitions)*
    )
}

fn message_name(typ: &Ty, fn_name: &Ident) -> Ident {
    if let &Ty::Path(_, Path { ref segments, .. }) = typ {
        let path = segments
            .iter()
            .map(|s| s.ident.as_ref())
            .collect::<Vec<_>>()
            .join("_");
        Ident::new(format!("MSG_{}_{}", path, fn_name))
    } else {
        unimplemented!()
    }
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
