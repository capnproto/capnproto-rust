use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
// use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Index};
use syn::{DataEnum, Fields, Ident, Path, Variant};

use heck::SnakeCase;

use crate::util::{is_data, is_primitive};

fn gen_type_write(variant: &Variant) -> TokenStream {
    // let variant_ident = &variant.ident;
    let variant_name = &variant.ident;
    let variant_snake_name = variant_name.to_string().to_snake_case();

    match &variant.fields {
        Fields::Unnamed(fields_unnamed) => {
            let unnamed = &fields_unnamed.unnamed;
            if unnamed.len() != 1 {
                unimplemented!();
            }

            let pair = unnamed.last().unwrap();
            let last_ident = match pair {
                syn::punctuated::Pair::End(last_ident) => last_ident,
                _ => unreachable!(),
            };

            let path = match &last_ident.ty {
                syn::Type::Path(type_path) => &type_path.path,
                _ => unimplemented!(),
            };

            if is_primitive(path) || is_data(path) {
                let set_method =
                    syn::Ident::new(&format!("set_{}", &variant_snake_name), variant.span());
                return quote! {
                    #variant_name(x) => writer.#set_method(x.clone()),
                };
            }

            // TODO: Deal with the case of list here

            let init_method =
                syn::Ident::new(&format!("init_{}", &variant_snake_name), variant.span());
            quote! {
                #variant_name(x) => x.write_capnp(&mut writer.reborrow().#init_method()),
            }
        }

        Fields::Unit => {
            let set_method =
                syn::Ident::new(&format!("set_{}", &variant_snake_name), variant.span());
            quote! {
                #variant_name => writer.#set_method(()),
            }
        }
        // Rust enum variants don't have named fields (?)
        Fields::Named(_) => unreachable!(),
    }
}

#[allow(unused)]
pub fn gen_write_capnp_enum(
    data_enum: &DataEnum,
    rust_enum: &Ident,
    capnp_struct: &Path,
) -> TokenStream {
    let recurse = data_enum.variants.iter().map(|variant| {
        let type_write = gen_type_write(&variant);
        quote! {
            #rust_enum::#type_write
        }
    });

    quote! {
        impl<'a> WriteCapnp<'a> for #rust_enum {
            type WriterType = #capnp_struct::Builder<'a>;
            fn write_capnp(&self, writer: &mut Self::WriterType) {
                match &self {
                    #(#recurse)*
                };
            }
        }
    }
}

fn gen_type_read(variant: &Variant, rust_enum: &Ident) -> TokenStream {
    let variant_name = &variant.ident;
    // let variant_snake_name = variant_name.to_string().to_snake_case();

    match &variant.fields {
        Fields::Unnamed(fields_unnamed) => {
            let unnamed = &fields_unnamed.unnamed;
            if unnamed.len() != 1 {
                unimplemented!();
            }

            let pair = unnamed.last().unwrap();
            let last_ident = match pair {
                syn::punctuated::Pair::End(last_ident) => last_ident,
                _ => unreachable!(),
            };

            let path = match &last_ident.ty {
                syn::Type::Path(type_path) => &type_path.path,
                _ => unimplemented!(),
            };

            if is_primitive(path) || is_data(path) {
                return quote! {
                    #variant_name(x) => #rust_enum::#variant_name(x),
                };
            }

            // TODO: Deal with the case of list here

            quote! {
                #variant_name(variant_reader) => {
                    // let variant_reader = variant_reader).into_result()?;
                    #rust_enum::#variant_name(#path::read_capnp(&variant_reader?)?)
                },
            }
        }

        Fields::Unit => {
            quote! {
                #variant_name(()) => #rust_enum::#variant_name,
            }
        }
        // Rust enum variants don't have named fields (?)
        Fields::Named(_) => unreachable!(),
    }
}

pub fn gen_read_capnp_enum(
    data_enum: &DataEnum,
    rust_enum: &Ident,
    capnp_struct: &Path,
) -> TokenStream {
    let recurse = data_enum.variants.iter().map(|variant| {
        let type_read = gen_type_read(&variant, rust_enum);
        quote! {
            #capnp_struct::#type_read
        }
    });

    quote! {
        impl<'a> ReadCapnp<'a> for #rust_enum {
            type ReaderType = #capnp_struct::Reader<'a>;

            fn read_capnp(reader: &Self::ReaderType) -> Result<Self, CapnpConvError> {
                Ok(match reader.which()? {
                    #(#recurse)*
                })
            }
        }
    }
}
