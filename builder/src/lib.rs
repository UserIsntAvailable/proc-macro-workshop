use std::ops::Not;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, AngleBracketedGenericArguments, Data, DataStruct, DeriveInput, Fields,
    FieldsNamed, GenericArgument, Ident, Meta, MetaNameValue, Path, PathArguments, PathSegment,
    Type, TypePath,
};

/// Information about the fields that are going to be generated.
#[derive(Debug)]
struct GenField {
    ident: Ident,
    ty: Type,
    is_optional: bool,
    each: Option<Ident>,
}

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput { ident, data, .. } = parse_macro_input!(input as DeriveInput);
    let fields = if let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = data
    {
        named
            .into_iter()
            .map(|field| {
                let (ty, is_optional) = if let Type::Path(TypePath {
                    qself: None,
                    path: Path { ref segments, .. },
                }) = field.ty
                {
                    #[rustfmt::skip]
                    let PathSegment { ident, arguments: args }
                        = segments.first().expect("has items");

                    if ident != "Option" {
                        (field.ty, false)
                    } else {
                        #[rustfmt::skip]
                        let PathArguments::AngleBracketed(
                            AngleBracketedGenericArguments { args, .. }
                        ) = args else { unreachable!() };

                        #[rustfmt::skip]
                        let GenericArgument::Type(ty)
                            = args.first().expect("has items") else { unreachable!() };

                        (ty.clone(), true)
                    }
                } else {
                    // a type is expected, because a field ( in this position ) should have one.
                    unreachable!()
                };

                field.attrs.into_iter().find(|attr| {
                    if let Meta::NameValue(MetaNameValue {
                        path: Path { segments, .. },
                        value,
                        ..
                    }) = &attr.meta
                    {
                        true
                    } else {
                        false
                    }
                });

                GenField {
                    ident: field.ident.expect("named fields"),
                    ty,
                    is_optional,
                    each: None,
                }
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
        quote! { #ident: std::option::Option::None }
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
                self.#ident = std::option::Option::Some(#ident);
                self
            }
        }
    });

    let init_struct_fields = fields.iter().map(
        |GenField {
             ident, is_optional, ..
         }| {
            let mut init = quote! { #ident: self.#ident.take() };

            if is_optional.not() {
                let error_msg = format!("The field `{}` was not setted.", ident);
                init = quote! {
                    #init.ok_or_else(|| std::string::String::from(#error_msg))?
                };
            }

            init
        },
    );

    quote! {
        impl #builder_ident {
            #(#setters)*

            pub fn build(
                &mut self
            ) -> std::result::Result<#buildee_ident, std::boxed::Box<dyn std::error::Error>> {
                std::result::Result::Ok(#buildee_ident {
                    #(#init_struct_fields),*
                })
            }
        }
    }
}
