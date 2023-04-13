use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields, FieldsNamed, Ident, Type};

/// Information about the fields that are going to be generated.
struct GenField {
    ident: Ident,
    ty: Type,
}

#[proc_macro_derive(Builder)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input as DeriveInput);
    let fields = if let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = data
    {
        named
            .into_iter()
            .map(|field| GenField {
                ident: field.ident.expect("named fields"),
                ty: field.ty,
            })
            .collect::<Vec<_>>()
    } else {
        unreachable!()
    };

    let builder_ident = format_ident!("{ident}Builder");
    let builder_struct = gen_builder_struct(&builder_ident, &fields);
    let buildee_impl = gen_buildee_impl(&ident, &builder_ident, &fields);
    let builder_impl = gen_builder_impl(&ident, &builder_ident, &fields);

    quote! {
        #builder_struct
        #buildee_impl
        #builder_impl
    }
    .into()
}

fn gen_builder_struct(ident: &Ident, fields: &[GenField]) -> TokenStream {
    let fields = fields.iter().map(|GenField { ident, ty, .. }| {
        quote! { #ident: std::option::Option<#ty> }
    });

    quote! {
        pub struct #ident {
            #(#fields),*
        }
    }
}

fn gen_buildee_impl(
    buildee_ident: &Ident,
    builder_ident: &Ident,
    fields: &[GenField],
) -> TokenStream {
    let init_struct_fields = fields.iter().map(|GenField { ident, .. }| {
        quote! { #ident: None }
    });

    quote! {
        impl #buildee_ident {
            pub fn builder() -> #builder_ident {
                #builder_ident {
                    #(#init_struct_fields),*
                }
            }
        }
    }
}

fn gen_builder_impl(
    buildee_ident: &Ident,
    builder_ident: &Ident,
    fields: &[GenField],
) -> TokenStream {
    let setters = fields.iter().map(|GenField { ident, ty, .. }| {
        quote! {
            fn #ident(&mut self, #ident: #ty) -> &mut Self {
                self.#ident = Some(#ident);
                self
            }
        }
    });

    let init_struct_fields = fields.iter().map(|GenField { ident, .. }| {
        let error_msg = format!("The field `{}` was not setted.", ident);
        quote! {
            #ident: self.#ident
                .take()
                .ok_or_else(|| String::from(#error_msg))?
        }
    });

    quote! {
        impl #builder_ident {
            #(#setters)*

            pub fn build(&mut self) -> Result<#buildee_ident, Box<dyn std::error::Error>> {
                std::result::Result::Ok(#buildee_ident {
                    #(#init_struct_fields),*
                })
            }
        }
    }
}
