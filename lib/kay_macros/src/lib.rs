#![feature(proc_macro, proc_macro_lib)]
#![recursion_limit="100"]

extern crate proc_macro;
use proc_macro::TokenStream;

extern crate syn;

#[macro_use]
extern crate quote;

#[proc_macro_derive(Compact)]
pub fn num_fields(input: TokenStream) -> TokenStream {
    let source = input.to_string();

    // Parse the string representation to an AST
    let ast = syn::parse_macro_input(&source).unwrap();

    // Build the output
    let expanded = expand_num_fields(&ast);

    // Return the original input struct unmodified, and the
    // generated impl along with it
    quote!(#ast #expanded).to_string().parse().unwrap()
}

fn expand_num_fields(ast: &syn::MacroInput) -> quote::Tokens {
    let fields : Vec<_> = match ast.body {
        syn::Body::Struct(ref data) => data.fields().iter().map(|ref f| &f.ident).collect(),
        syn::Body::Enum(_) => panic!("#[derive(Compact)] can't be used with enums yet :('"),
    };

    let fields_ref = &fields;
    let fields_ref2 = &fields;

    // Used in the quasi-quotation below as `#name`
    let name = &ast.ident;

    // Helper is provided for handling complex generic types correctly and effortlessly
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    quote! {
        // The generated impl
        impl #impl_generics ::kay::Compact for #name #ty_generics #where_clause {
            fn is_still_compact(&self) -> bool {
                #(self.#fields_ref.is_still_compact())&&*
            }

            fn dynamic_size_bytes(&self) -> usize {
                #(self.#fields_ref.dynamic_size_bytes())+*
            }

            unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
                #![allow(unused_assignments)]
                let mut offset: isize = 0;
                #(
                    let source_field = &source.#fields_ref2;
                    self.#fields_ref.compact_from(&source_field, new_dynamic_part.offset(offset));
                    offset += source_field.dynamic_size_bytes() as isize;
                )*
            }
        }
    }
}