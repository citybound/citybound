//! Automatic `#[derive(Compact)]` macro for structs whose fields are all `Compact`

#![recursion_limit="100"]

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

fn get_field_idents(variant_data: &syn::VariantData,
                    tuple_prefix: &'static str)
                    -> Vec<syn::Ident> {
    match *variant_data {
        syn::VariantData::Tuple(ref fields) => {
            fields.iter()
                .enumerate()
                .map(|(i, _f)| format!("{}{}", tuple_prefix, i).into())
                .collect()
        }
        syn::VariantData::Unit => Vec::new(),
        syn::VariantData::Struct(_) => panic!("struct variants in enums not supported yet"),
    }
}

fn expand_derive_compact(ast: &syn::MacroInput) -> quote::Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let tokens = match ast.body {
        syn::Body::Struct(ref data) => {
            let fields: Vec<_> = data.fields()
                .iter()
                .enumerate()
                .map(|(i, ref f)| f.ident.clone().unwrap_or(format!("{}", i).into()))
                .collect();
            let fields_ref = &fields;
            let fields_ref2 = &fields;

            let decompact_body = if data.fields()[0].ident.is_some() {
                quote! {
                    #name{
                        #(
                            #fields_ref: self.#fields_ref2.decompact()
                        ),*
                    }
                }
            } else {
                quote! {
                    #name(#(self.#fields_ref2.decompact()),*)
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
                    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
                        let mut offset: isize = 0;
                        #(
                            let source_field = &source.#fields_ref2;
                            self.#fields_ref.compact_from(source_field,
                                                          new_dynamic_part.offset(offset));
                            offset += source_field.dynamic_size_bytes() as isize;
                        )*
                    }

                    unsafe fn decompact(&self) -> Self {
                        #decompact_body
                    }
                }
            }
        }
        syn::Body::Enum(ref data) => {
            let variants_still_compact: &Vec<_> = &data.iter()
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

            let variants_dynamic_size: &Vec<_> = &data.iter()
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

            let variants_compact_from: &Vec<_> = &data.iter()
                .map(|variant| {
                    let ident = &variant.ident;
                    let fields = get_field_idents(&variant.data, "f");
                    let source_fields = get_field_idents(&variant.data, "source_f");
                    let fields_ref = &fields;
                    let source_fields_ref = &source_fields;
                    let source_fields_ref_2 = &source_fields;

                    if fields.is_empty() {
                        quote! {
                            #name::#ident => {
                                *self = #name::#ident;
                            }
                        }
                    } else {
                        quote! {
                            #name::#ident(#(ref #source_fields_ref),*) => {
                                ::std::ptr::copy_nonoverlapping(source as *const #name,
                                                                self as *mut #name, 1);
                                let mut offset: isize = 0;
                                if let #name::#ident(#(ref mut #fields_ref),*) = *self {
                                    #(
                                        #fields_ref.compact_from(#source_fields_ref,
                                                                new_dynamic_part
                                                                    .offset(offset));
                                        offset += #source_fields_ref_2.dynamic_size_bytes() as isize;
                                    )*
                                } else {unreachable!()}
                            }
                        }
                    }
                })
                .collect();

            let variants_decompact: &Vec<_> = &data.iter()
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
                                #name::#ident(#(#fields_ref.decompact()),*)
                            }
                        }
                    }
                })
                .collect();

            quote! {
                // generated
                impl #impl_generics ::compact::Compact for #name #ty_generics #where_clause {
                    #[allow(match_same_arms)]
                    fn is_still_compact(&self) -> bool {
                        match *self {
                            #(#variants_still_compact),*
                        }
                    }

                    #[allow(match_same_arms)]
                    fn dynamic_size_bytes(&self) -> usize {
                        match *self {
                            #(#variants_dynamic_size),*
                        }
                    }

                    #[allow(unused_assignments)]
                    #[allow(match_same_arms)]
                    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
                        match *source {
                            #(#variants_compact_from),*
                        }
                    }

                    #[allow(match_same_arms)]
                    unsafe fn decompact(&self) -> Self {
                        match *self {
                            #(#variants_decompact),*
                        }
                    }
                }
            }
        }
    };

    tokens
}
