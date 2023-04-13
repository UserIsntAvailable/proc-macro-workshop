use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed};

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input as DeriveInput);

    let builder_ident = format_ident!("{ident}Builder");
    let (struct_fields, init_fields, setters, build_method_init_fields) =
        if let Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) = data
        {
            // TODO: Is it a good practice to directly put an `Option` inside `quote!` if I now they
            // can't be `None`?
            // TODO: Split different sections on different functions.
            // TODO: return `named` from `if let` to remove unnecessary nesting.
            // FIX: Stop collecting `vectors` and use the `#(...)*` syntax to use iterators.

            let struct_fields = named
                .iter()
                .map(|Field { ident, ty, .. }| {
                    quote! { #ident: std::option::Option<#ty>, }
                })
                .collect::<TokenStream>();

            let init_fields = named
                .iter()
                .map(|Field { ident, .. }| {
                    quote! { #ident: None, }
                })
                .collect::<TokenStream>();

            let setters = named
                .iter()
                .map(|Field { ident, ty, .. }| {
                    quote! {
                        fn #ident(&mut self, #ident: #ty) -> &mut Self {
                            self.#ident = Some(#ident);
                            self
                        }
                    }
                })
                .collect::<TokenStream>();

            let build_method_init_fields = named
                .iter()
                .map(|Field { ident, .. }| {
                    let error_msg =
                        format!("The field `{}` was not setted.", ident.as_ref().unwrap());
                    quote! {
                        #ident: self.#ident
                            .take()
                            .ok_or_else(|| String::from(#error_msg))?,
                    }
                })
                .collect::<TokenStream>();

            (
                struct_fields,
                init_fields,
                setters,
                build_method_init_fields,
            )
        } else {
            unreachable!()
        };

    quote! {
        pub struct #builder_ident {
            #struct_fields
        }

        impl #ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #init_fields
                }
            }
        }

        impl #builder_ident {
            #setters

            pub fn build(&mut self) -> Result<#ident, Box<dyn std::error::Error>> {
                std::result::Result::Ok(#ident {
                    #build_method_init_fields
                })
            }
        }
    }
    .into()
}
