#![recursion_limit="100"]

extern crate proc_macro;
use proc_macro::TokenStream;

extern crate syn;

#[macro_use]
extern crate quote;

//
// DEBUG MACRO EXPANSION LIKE THIS:
// cargo rustc -- -Z unstable-options --pretty=expanded > output.rs
//

#[proc_macro_derive(Actor)]
pub fn derive_actor(input: TokenStream) -> TokenStream {
    let source = input.to_string();

    // Parse the string representation to an AST
    let ast = syn::parse_macro_input(&source).unwrap();

    // Build the output
    let expanded = expand_derive_actor(&ast);

    // Return the original input struct unmodified, and the
    // generated impl along with it
    quote!(#expanded).to_string().parse().unwrap()
}

fn expand_derive_actor(ast: &syn::MacroInput) -> quote::Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote! {
        // generated
        impl #impl_generics ::kay::Actor for #name #ty_generics #where_clause {
            fn id(&self) -> ::kay::ID {
                self._id.expect("ID not set")
            }

            unsafe fn set_id(&mut self, id: ::kay::ID) {
                self._id = Some(id);
            }
        }
    }
}

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

fn expand_derive_compact(ast: &syn::MacroInput) -> quote::Tokens {
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let tokens = match ast.body {
        syn::Body::Struct(ref data) => {
            let fields : Vec<_> = data.fields().iter().map(|ref f| &f.ident).collect();
            let fields_ref = &fields;
            let fields_ref2 = &fields;    

            quote! {
                // generated
                impl #impl_generics ::kay::Compact for #name #ty_generics #where_clause {
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
                            self.#fields_ref.compact_from(source_field, new_dynamic_part.offset(offset));
                            offset += source_field.dynamic_size_bytes() as isize;
                        )*
                    }

                    unsafe fn decompact(&self) -> Self {
                        #name{
                            #(
                                #fields_ref: self.#fields_ref2.decompact()
                            ),*
                        }
                    }
                }
            }
        }
        syn::Body::Enum(ref data) => {
            let variants_still_compact : &Vec<_> = &data.iter().map(|variant| {
                if let syn::VariantData::Tuple(ref fields) = variant.data {
                    let ident = &variant.ident;
                    let fields : Vec<syn::Ident> = fields.iter().enumerate().map(|(i, _f)| format!("f{}", i).into()).collect();
                    let fields_ref = &fields;
                    quote! {
                        #name::#ident(#(ref #fields_ref),*) => {
                            #(#fields_ref.is_still_compact())&&*
                        }
                    }
                } else {panic!("Only tuple enum variants supported so far");}
            }).collect();

            let variants_dynamic_size : &Vec<_> = &data.iter().map(|variant| {
                if let syn::VariantData::Tuple(ref fields) = variant.data {
                    let ident = &variant.ident;
                    let fields : Vec<syn::Ident> = fields.iter().enumerate().map(|(i, _f)| format!("f{}", i).into()).collect();
                    let fields_ref = &fields;
                    quote! {
                        #name::#ident(#(ref #fields_ref),*) => {
                            #(#fields_ref.dynamic_size_bytes())+*
                        }
                    }
                } else {panic!("Only tuple enum variants supported so far");}
            }).collect();

            let variants_compact_from : &Vec<_> = &data.iter().map(|variant| {
                if let syn::VariantData::Tuple(ref fields) = variant.data {
                    let ident = &variant.ident;
                    let fields : Vec<syn::Ident> = fields.iter().enumerate().map(|(i, _f)| format!("f{}", i).into()).collect();
                    let source_fields : Vec<syn::Ident> = fields.iter().enumerate().map(|(i, _f)| format!("source_f{}", i).into()).collect();
                    let fields_ref = &fields;
                    let source_fields_ref = &source_fields;
                    let source_fields_ref_2 = &source_fields;
                    quote! {
                        #name::#ident(#(ref #source_fields_ref),*) => {
                            ::std::ptr::copy_nonoverlapping(source as *const #name, self as *mut #name, 1);
                            let mut offset: isize = 0;
                            if let #name::#ident(#(ref mut #fields_ref),*) = *self {
                                #(
                                    #fields_ref.compact_from(#source_fields_ref, new_dynamic_part.offset(offset));
                                    offset += #source_fields_ref_2.dynamic_size_bytes() as isize;
                                )*
                            } else {unreachable!()}
                        }
                    }
                } else {panic!("Only tuple enum variants supported so far");}
            }).collect();

            let variants_decompact : &Vec<_> = &data.iter().map(|variant| {
                if let syn::VariantData::Tuple(ref fields) = variant.data {
                    let ident = &variant.ident;
                    let fields : Vec<syn::Ident> = fields.iter().enumerate().map(|(i, _f)| format!("f{}", i).into()).collect();
                    let fields_ref = &fields;
                    quote! {
                        #name::#ident(#(ref #fields_ref),*) => {
                            #name::#ident(#(#fields_ref.decompact()),*)
                        }
                    }
                } else {panic!("Only tuple enum variants supported so far");}
            }).collect();

            quote! {
                // generated
                impl #impl_generics ::kay::Compact for #name #ty_generics #where_clause {
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
        },
    };

    tokens
}