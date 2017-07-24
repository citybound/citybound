use syn::*;
use {Model, TraitName, Handler, HandlerScope};

pub fn parse(file: &str) -> Model {
    let mut model = Model::default();

    for item in parse_crate(file).unwrap().items.iter() {
        let ident = &item.ident;
        match item.node {
            ItemKind::Struct(_, _) => {
                let ident_as_seg: PathSegment = ident.clone().into();
                let actor_def = model
                    .actors
                    .entry(Ty::Path(None, ::syn::Path::from(ident_as_seg)))
                    .or_insert_with(Default::default);
                actor_def.defined_here = true;
            }
            ItemKind::Impl(_, _, _, ref maybe_trait, ref actor_name, ref impl_items) => {
                let actor_def = model.actors.entry((**actor_name).clone()).or_insert_with(
                    Default::default,
                );
                let actor_path = match **actor_name {
                    Ty::Path(_, ref path) => path,
                    _ => unimplemented!(),
                };
                if let Some(ref trait_name) = *maybe_trait {
                    actor_def.impls.push(trait_name.clone());
                    actor_def.handlers.extend(handlers_from_impl_items(
                        impl_items,
                        Some(trait_name.clone()),
                        actor_path,
                    ));
                } else {
                    actor_def.handlers.extend(handlers_from_impl_items(
                        impl_items,
                        None,
                        actor_path,
                    ));
                }
            }
            ItemKind::Trait(_, _, _, ref trait_items) => {
                let trait_name: TraitName = ::syn::Path::from(PathSegment::from(ident.clone()));
                let trait_def = model.traits.entry(trait_name.clone()).or_insert_with(
                    Default::default,
                );
                let as_segment: PathSegment = ident.clone().into();
                trait_def.handlers.extend(handlers_from_trait_items(
                    trait_items,
                    &::syn::Path::from(as_segment),
                ));
            }
            _ => {}
        }
    }

    model.actors.retain(|ref _name, ref actor_def| {
        !actor_def.handlers.is_empty()
    });

    model.traits.retain(|ref _name, ref trait_def| {
        !trait_def.handlers.is_empty()
    });

    {
        let traits = &model.traits;
        let actors = &mut model.actors;
        for actor_def in actors.values_mut() {
            actor_def.impls.retain(
                |trait_name| traits.get(trait_name).is_some(),
            );
        }
    }

    model
}

fn handlers_from_impl_items(
    impl_items: &[ImplItem],
    with_trait: Option<TraitName>,
    parent_path: &::syn::Path,
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
                handler_from(fn_name, sig, attrs, with_trait.clone(), parent_path)
            } else {
                None
            }
        } else {
            None
        })
        .collect()
}

fn handlers_from_trait_items(trait_items: &[TraitItem], parent_path: &::syn::Path) -> Vec<Handler> {
    trait_items
        .iter()
        .filter_map(|trait_item| if let &TraitItem {
            ident: ref fn_name,
            node: TraitItemKind::Method(ref sig, _),
            ..
        } = trait_item
        {
            handler_from(fn_name, sig, &[], None, parent_path)
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
    parent_path: &::syn::Path,
) -> Option<Handler> {
    check_handler(sig, parent_path.clone()).and_then(|(args, scope)| {
        let returns_fate = match sig.decl.output {
            FunctionRetTy::Default => false,
            FunctionRetTy::Ty(Ty::Path(_, Path { ref segments, .. })) => {
                segments.iter().any(|s| s.ident.as_ref() == "Fate")
            }
            _ => return None,
        };

        let is_critical = attrs.iter().any(|attr| {
            attr.is_sugared_doc &&
                attr.value == MetaItem::NameValue("doc".into(), "/// Critical".into())
        });

        Some(Handler {
            name: fn_name.clone(),
            arguments: args.to_vec(),
            scope: scope,
            critical: is_critical,
            returns_fate: returns_fate,
            from_trait: from_trait.clone(),
        })
    })
}

pub fn check_handler(
    sig: &MethodSig,
    parent_path: ::syn::Path,
) -> Option<(&[FnArg], HandlerScope)> {
    if let Some(&FnArg::Captured(_, Ty::Rptr(_, ref ty_box))) = sig.decl.inputs.last() {
        if let &MutTy {
            mutability: Mutability::Mutable,
            ty: Ty::Path(_, ref path),
        } = &**ty_box
        {
            if path.segments.last().unwrap().ident == Ident::new("World") {
                match sig.decl.inputs.get(0) {
                    Some(&FnArg::SelfRef(_, _)) => {
                        let args = &sig.decl.inputs[1..(sig.decl.inputs.len() - 1)];
                        Some((args, HandlerScope::SubActor))
                    }
                    Some(&FnArg::SelfValue(_)) => None,
                    _ => {
                        let self_segment: PathSegment = Ident::new("Self").into();
                        match sig.decl.output {
                            FunctionRetTy::Ty(Ty::Path(_, ref ret_ty_path))
                                if *ret_ty_path == ::syn::Path::from(self_segment) ||
                                       *ret_ty_path == parent_path => {
                                let args = &sig.decl.inputs[1..(sig.decl.inputs.len() - 1)];
                                Some((args, HandlerScope::Init))
                            }
                            _ => {
                                let args = &sig.decl.inputs[0..(sig.decl.inputs.len() - 1)];
                                Some((args, HandlerScope::Swarm))
                            }
                        }
                    }
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