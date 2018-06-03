//! Automatic `#[derive(Compact)]` macro for structs whose fields are all `Compact`

#![recursion_limit = "256"]

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

extern crate proc_macro;
use proc_macro::TokenStream;

extern crate syn;

#[macro_use]
extern crate quote;

// DEBUG MACRO EXPANSION LIKE THIS:
// cargo rustc -- -Z unstable-options --pretty=expanded > output.rs
//

#[proc_macro_derive(Compact)]
pub fn derive_compact(input: TokenStream) -> TokenStream {
    let source = input.to_string();

    // Parse the string representation to an AST
    let ast = syn::parse_macro_input(&source).unwrap();

    // Build the output
    let expanded = expand_derive_compact(&ast);

    // Return the original input struct unmodified, and the
    // generated impl along with it
    quote!(#expanded).to_string().parse().unwrap()
}

fn get_field_idents(
    variant_data: &syn::VariantData,
    tuple_prefix: &'static str,
) -> Vec<syn::Ident> {
    match *variant_data {
        syn::VariantData::Tuple(ref fields) => fields
            .iter()
            .enumerate()
            .map(|(i, _f)| format!("{}{}", tuple_prefix, i).into())
            .collect(),
        syn::VariantData::Unit => Vec::new(),
        syn::VariantData::Struct(_) => panic!("struct variants in enums not supported yet"),
    }
}

fn expand_derive_compact(ast: &syn::MacroInput) -> quote::Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    match ast.body {
        syn::Body::Struct(ref data) => {
            let fields: Vec<_> = data
                .fields()
                .iter()
                .enumerate()
                .map(|(i, ref f)| f.ident.clone().unwrap_or_else(|| format!("{}", i).into()))
                .collect();
            let fields_ref = &fields;
            let fields_ref2 = &fields;
            let fields_ref3 = &fields;

            let decompact_body = if data.fields()[0].ident.is_some() {
                quote! {
                    #name{
                        #(
                            #fields_ref: ::compact::Compact::decompact(&(*source).#fields_ref2)
                        ),*
                    }
                }
            } else {
                quote! {
                    #name(#(::compact::Compact::decompact(&(*source).#fields_ref2)),*)
                }
            };

            quote! {
                // generated
                impl #impl_generics ::compact::Compact for #name #ty_generics #where_clause {
                    fn is_still_compact(&self) -> bool {
                        #(self.#fields_ref.is_still_compact())&&*
                    }

                    fn dynamic_size_bytes(&self) -> usize {
                        #(self.#fields_ref.dynamic_size_bytes())+*
                    }

                    #[allow(unused_assignments)]
                    unsafe fn compact(
                        source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8
                    ) {
                        let mut offset: isize = 0;
                        #(
                            let size_of_this_field = (*source).#fields_ref
                                                                .dynamic_size_bytes() as isize;
                            ::compact::Compact::compact(
                                &mut (*source).#fields_ref2,
                                &mut (*dest).#fields_ref3,
                                new_dynamic_part.offset(offset)
                            );
                            offset += size_of_this_field;
                        )*
                    }

                    unsafe fn decompact(source: *const Self) -> Self {
                        #decompact_body
                    }
                }
            }
        }
        syn::Body::Enum(ref data) => {
            let variants_still_compact: &Vec<_> = &data
                .iter()
                .map(|variant| {
                    let ident = &variant.ident;
                    let fields = get_field_idents(&variant.data, "f");
                    let fields_ref = &fields;

                    if fields.is_empty() {
                        quote! {
                            #name::#ident => true
                        }
                    } else {
                        quote! {
                            #name::#ident(#(ref #fields_ref),*) => {
                                #(#fields_ref.is_still_compact())&&*
                            }
                        }
                    }
                })
                .collect();

            let variants_dynamic_size: &Vec<_> = &data
                .iter()
                .map(|variant| {
                    let ident = &variant.ident;
                    let fields = get_field_idents(&variant.data, "f");
                    let fields_ref = &fields;

                    if fields.is_empty() {
                        quote! {
                            #name::#ident => 0
                        }
                    } else {
                        quote! {
                            #name::#ident(#(ref #fields_ref),*) => {
                                #(#fields_ref.dynamic_size_bytes())+*
                            }
                        }
                    }
                })
                .collect();

            let variants_compact_to: &Vec<_> = &data
                .iter()
                .map(|variant| {
                    let ident = &variant.ident;
                    let fields = get_field_idents(&variant.data, "f");
                    let dest_fields = get_field_idents(&variant.data, "dest_f");
                    let fields_ref = &fields;
                    let fields_ref2 = &fields;
                    let dest_fields_ref = &dest_fields;

                    if fields.is_empty() {
                        quote! {
                            #name::#ident => {}
                        }
                    } else {
                        quote! {
                            #name::#ident(#(ref mut #fields_ref),*) => {
                                let mut offset: isize = 0;
                                if let #name::#ident(#(ref mut #dest_fields_ref),*) = *dest {
                                    #(
                                        let size_of_this_field = #fields_ref
                                                                .dynamic_size_bytes() as isize;
                                        ::compact::Compact::compact(
                                            #fields_ref2,
                                            #dest_fields_ref,
                                            new_dynamic_part.offset(offset)
                                        );
                                        offset += size_of_this_field;
                                    )*
                                  } else {unreachable!()}
                            }
                        }
                    }
                })
                .collect();

            let variants_decompact: &Vec<_> = &data
                .iter()
                .map(|variant| {
                    let ident = &variant.ident;
                    let fields = get_field_idents(&variant.data, "f");
                    let fields_ref = &fields;

                    if fields.is_empty() {
                        quote! {
                            #name::#ident => #name::#ident
                        }
                    } else {
                        quote! {
                            #name::#ident(#(ref #fields_ref),*) => {
                                #name::#ident(#(::compact::Compact::decompact(#fields_ref)),*)
                            }
                        }
                    }
                })
                .collect();

            quote! {
                // generated
                impl #impl_generics ::compact::Compact for #name #ty_generics #where_clause {
                    #[cfg_attr(feature = "cargo-clippy", allow(match_same_arms))]
                    fn is_still_compact(&self) -> bool {
                        match *self {
                            #(#variants_still_compact),*
                        }
                    }

                    #[cfg_attr(feature = "cargo-clippy", allow(match_same_arms))]
                    fn dynamic_size_bytes(&self) -> usize {
                        match *self {
                            #(#variants_dynamic_size),*
                        }
                    }

                    #[allow(unused_assignments)]
                    #[cfg_attr(feature = "cargo-clippy", allow(match_same_arms))]
                    unsafe fn compact(
                        source: *mut Self,
                        dest: *mut Self,
                        new_dynamic_part: *mut u8
                    ) {
                        ::std::ptr::copy_nonoverlapping(source, dest, 1);
                        match *source {
                            #(#variants_compact_to),*
                        }
                    }

                    #[cfg_attr(feature = "cargo-clippy", allow(match_same_arms))]
                    unsafe fn decompact(source: *const Self) -> Self {
                        match *source {
                            #(#variants_decompact),*
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn basic_struct() {
    let input = quote!(struct Test {
        number: u32,
        list: CVec<f32>,
        truth: bool,
    });

    let expected = quote!(
        impl ::compact::Compact for Test {
            fn is_still_compact(&self) -> bool {
                self.number.is_still_compact() &&
                    self.list.is_still_compact() &&
                    self.truth.is_still_compact()
            }

            fn dynamic_size_bytes(&self) -> usize {
                self.number.dynamic_size_bytes() +
                    self.list.dynamic_size_bytes() +
                    self.truth.dynamic_size_bytes()
            }

            #[allow(unused_assignments)]
            unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
                let mut offset: isize = 0;
                let size_of_this_field = (*source).number.dynamic_size_bytes() as isize;
                ::compact::Compact::compact(
                    &mut (*source).number,
                    &mut (*dest).number,
                    new_dynamic_part.offset(offset)
                );
                offset += size_of_this_field;

                let size_of_this_field = (*source).list.dynamic_size_bytes() as isize;
                ::compact::Compact::compact(
                    &mut (*source).list,
                    &mut (*dest).list,
                    new_dynamic_part.offset(offset)
                );
                offset += size_of_this_field;

                let size_of_this_field = (*source).truth.dynamic_size_bytes() as isize;
                ::compact::Compact::compact(
                    &mut (*source).truth,
                    &mut (*dest).truth,
                    new_dynamic_part.offset(offset)
                );
                offset += size_of_this_field;
            }

            unsafe fn decompact(source: *const Self) -> Self {
                Test {
                    number: ::compact::Compact::decompact(&(*source).number),
                    list: ::compact::Compact::decompact(&(*source).list),
                    truth: ::compact::Compact::decompact(&(*source).truth)
                }
            }
        }
    );

    assert_eq!(
        expected.into_string(),
        expand_derive_compact(&syn::parse_macro_input(input.into_string().as_str()).unwrap())
            .into_string()
    );
}

#[test]
fn basic_enum() {
    let input = quote!(enum Test2 {
        A(u32, bool, f32),
        B(CVec<u8>),
        C,
    });

    let expected = quote!(
        impl ::compact::Compact for Test2 {
            #[allow(match_same_arms)]
            fn is_still_compact(&self) -> bool {
                match *self {
                    Test2::A(ref f0, ref f1, ref f2) => {
                        f0.is_still_compact()
                            && f1.is_still_compact() && f2.is_still_compact()
                    },
                    Test2::B(ref f0) => {
                        f0.is_still_compact()
                    },
                    Test2::C => true
                }
            }

            #[allow(match_same_arms)]
            fn dynamic_size_bytes(&self) -> usize {
                match *self {
                    Test2::A(ref f0, ref f1, ref f2) => {
                        f0.dynamic_size_bytes()
                            + f1.dynamic_size_bytes() + f2.dynamic_size_bytes()
                    },
                    Test2::B(ref f0) => {
                        f0.dynamic_size_bytes()
                    },
                    Test2::C => 0
                }
            }

            #[allow(unused_assignments)]
            #[allow(match_same_arms)]
            unsafe fn compact(source: *mut Self, dest: *mut Self, new_dynamic_part: *mut u8) {
                ::std::ptr::copy_nonoverlapping(source, dest, 1);
                match *source {
                    Test2::A(ref mut f0, ref mut f1, ref mut f2) => {
                        let mut offset: isize = 0;
                        if let Test2::A(
                            ref mut dest_f0, ref mut dest_f1, ref mut dest_f2
                        ) = *dest {
                            let size_of_this_field = f0.dynamic_size_bytes() as isize;
                            ::compact::Compact::compact(
                                f0, dest_f0, new_dynamic_part.offset(offset)
                            );
                            offset += size_of_this_field;

                            let size_of_this_field = f1.dynamic_size_bytes() as isize;
                            ::compact::Compact::compact(
                                f1, dest_f1, new_dynamic_part.offset(offset)
                            );
                            offset += size_of_this_field;

                            let size_of_this_field = f2.dynamic_size_bytes() as isize;
                            ::compact::Compact::compact(
                                f2, dest_f2, new_dynamic_part.offset(offset)
                            );
                            offset += size_of_this_field;
                        } else {unreachable!()}
                    },
                    Test2::B(ref mut f0) => {
                        let mut offset: isize = 0;
                        if let Test2::B(ref mut dest_f0) = *dest {
                            let size_of_this_field = f0.dynamic_size_bytes() as isize;
                            ::compact::Compact::compact(
                                f0, dest_f0, new_dynamic_part.offset(offset)
                            );
                            offset += size_of_this_field;
                        } else {unreachable!()}
                    },
                    Test2::C => {}
                }
            }

            #[allow(match_same_arms)]
            unsafe fn decompact(source: *const Self) -> Self {
                match *source {
                    Test2::A(ref f0, ref f1, ref f2) => {
                        Test2::A(
                            ::compact::Compact::decompact(f0),
                            ::compact::Compact::decompact(f1),
                            ::compact::Compact::decompact(f2)
                        )
                    },
                    Test2::B(ref f0) => {
                        Test2::B(::compact::Compact::decompact(f0))
                    },
                    Test2::C => Test2::C
                }
            }
        }
    );

    assert_eq!(
        expected.into_string(),
        expand_derive_compact(&syn::parse_macro_input(input.into_string().as_str()).unwrap())
            .into_string()
    );
}
