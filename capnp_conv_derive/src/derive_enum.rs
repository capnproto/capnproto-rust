use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
// use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Index};
use syn::{DataEnum, Fields, Ident, Path, Variant};

use heck::SnakeCase;

use crate::util::{
    gen_list_read_iter, gen_list_write_iter, get_list, is_data, is_primitive, usize_to_u32_shim,
};

fn gen_type_write(variant: &Variant, assign_defaults: impl Fn(&mut syn::Path)) -> TokenStream {
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

            let mut path = path.clone();
            assign_defaults(&mut path);

            if is_data(&path) {
                let set_method =
                    syn::Ident::new(&format!("set_{}", &variant_snake_name), variant.span());
                return quote! {
                    #variant_name(x) => writer.#set_method(x),
                };
            }

            if is_primitive(&path) || is_data(&path) {
                let set_method =
                    syn::Ident::new(&format!("set_{}", &variant_snake_name), variant.span());
                return quote! {
                    #variant_name(x) => writer.#set_method(x.clone()),
                };
            }

            // The case of list:
            if let Some(inner_path) = get_list(&path) {
                let init_method =
                    syn::Ident::new(&format!("init_{}", &variant_snake_name), variant.span());
                let list_write_iter = gen_list_write_iter(&inner_path);

                // In the cases of more complicated types, list_builder needs to be mutable.
                let let_list_builder =
                    if is_primitive(&path) || path.is_ident("String") || is_data(&path) {
                        quote! { let list_builder }
                    } else {
                        quote! { let mut list_builder }
                    };

                let usize_to_u32 = usize_to_u32_shim();
                return quote! {
                    #variant_name(vec) => {
                        #usize_to_u32
                        #let_list_builder = writer
                            .reborrow()
                            .#init_method(usize_to_u32(vec.len()).unwrap());

                        for (index, item) in vec.iter().enumerate() {
                            #list_write_iter
                        }
                    },
                };
            }

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

pub fn gen_write_capnp_enum(
    data_enum: &DataEnum,
    rust_enum: &Ident,
    capnp_struct: &Path,
    assign_defaults: impl Fn(&mut syn::Path),
) -> TokenStream {
    let recurse = data_enum.variants.iter().map(|variant| {
        let type_write = gen_type_write(&variant, &assign_defaults);
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

fn gen_type_read(
    variant: &Variant,
    rust_enum: &Ident,
    assign_defaults: impl Fn(&mut syn::Path),
) -> TokenStream {
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

            let mut path = path.clone();
            assign_defaults(&mut path);

            if is_data(&path) {
                return quote! {
                    #variant_name(x) => #rust_enum::#variant_name(x?.into()),
                };
            }

            if is_primitive(&path) {
                return quote! {
                    #variant_name(x) => #rust_enum::#variant_name(x),
                };
            }

            if let Some(inner_path) = get_list(&path) {
                // The case of a list:
                let list_read_iter = gen_list_read_iter(&inner_path);
                return quote! {
                    #variant_name(list_reader) => {
                        let mut res_vec = Vec::new();
                        for item_reader in list_reader? {
                            // res_vec.push_back(read_named_relay_address(&named_relay_address)?);
                            #list_read_iter
                        }
                        #rust_enum::#variant_name(res_vec)
                    }
                };
            }

            quote! {
                #variant_name(variant_reader) => {
                    // let variant_reader = variant_reader).into_result()?;
                    #rust_enum::#variant_name(<#path>::read_capnp(&variant_reader?)?)
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
    assign_defaults: impl Fn(&mut syn::Path),
) -> TokenStream {
    let recurse = data_enum.variants.iter().map(|variant| {
        let type_read = gen_type_read(&variant, rust_enum, &assign_defaults);
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
