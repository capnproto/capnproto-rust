#![crate_type = "lib"]
#![feature(nll)]
#![feature(generators)]
#![feature(never_type)]
#![deny(trivial_numeric_casts, warnings)]
#![allow(intra_doc_link_resolution_failure)]
#![allow(
    clippy::too_many_arguments,
    clippy::implicit_hasher,
    clippy::module_inception,
    clippy::new_without_default
)]
#![allow(unreachable_code)]

extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
// use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Index};
use syn::{parse_macro_input, Data, DataEnum, DeriveInput, Fields, FieldsNamed, Ident, Path};

/// Is a primitive type?
fn is_primitive(path: &syn::Path) -> bool {
    path.is_ident("u8")
        || path.is_ident("u16")
        || path.is_ident("u32")
        || path.is_ident("u64")
        || path.is_ident("i8")
        || path.is_ident("i16")
        || path.is_ident("i32")
        || path.is_ident("i64")
        || path.is_ident("bool")
}

/// Check if the path represents a Vec<u8>
fn is_data(path: &syn::Path) -> bool {
    let last_segment = match path.segments.last().unwrap() {
        syn::punctuated::Pair::End(last_segment) => last_segment,
        _ => unreachable!(),
    };
    if &last_segment.ident.to_string() != "Vec" {
        return false;
    }
    let angle = match &last_segment.arguments {
        syn::PathArguments::AngleBracketed(angle) => {
            if angle.args.len() > 1 {
                unreachable!("Too many arguments for Vec!");
            }
            angle
        }
        _ => unreachable!("Vec with arguments that are not angle bracketed!"),
    };
    let last_arg = match angle.args.last().unwrap() {
        syn::punctuated::Pair::End(last_arg) => last_arg,
        _ => return false,
    };

    let arg_ty = match last_arg {
        syn::GenericArgument::Type(arg_ty) => arg_ty,
        _ => return false,
    };

    let arg_ty_path = match arg_ty {
        syn::Type::Path(arg_ty_path) => arg_ty_path,
        _ => return false,
    };

    if arg_ty_path.qself.is_some() {
        return false;
    }

    if !arg_ty_path.path.is_ident("u8") {
        return false;
    }

    true
}

/// Check if the path represents a Vec<SomeStruct>, where SomeStruct != u8
fn get_list(path: &syn::Path) -> Option<syn::Path> {
    let last_segment = match path.segments.last().unwrap() {
        syn::punctuated::Pair::End(last_segment) => last_segment,
        _ => unreachable!(),
    };
    if &last_segment.ident.to_string() != "Vec" {
        return None;
    }
    let angle = match &last_segment.arguments {
        syn::PathArguments::AngleBracketed(angle) => {
            if angle.args.len() > 1 {
                unreachable!("Too many arguments for Vec!");
            }
            angle
        }
        _ => unreachable!("Vec with arguments that are not angle bracketed!"),
    };
    let last_arg = match angle.args.last().unwrap() {
        syn::punctuated::Pair::End(last_arg) => last_arg,
        _ => return None,
    };

    let arg_ty = match last_arg {
        syn::GenericArgument::Type(arg_ty) => arg_ty,
        _ => return None,
    };

    let arg_ty_path = match arg_ty {
        syn::Type::Path(arg_ty_path) => arg_ty_path,
        _ => return None,
    };

    if arg_ty_path.qself.is_some() {
        return None;
    }

    // Make sure that we don't deal with Vec<u8>:
    if arg_ty_path.path.is_ident("u8") {
        return None;
    }

    Some(arg_ty_path.path.clone())
}

fn gen_list_write_iter(path: &syn::Path) -> TokenStream {
    if is_primitive(path) || path.is_ident("String") || is_data(path) {
        // A primitive list:
        quote! {
            list_builder
                .reborrow()
                .set(u32::try_from(index).unwrap(), item.clone());
        }
    } else {
        // Not a primitive list:
        quote! {
            let mut item_builder = list_builder
                .reborrow()
                .get(u32::try_from(index).unwrap());

            item.write_capnp(&mut item_builder);
        }
    }
    // TODO: It seems like we do not support List(List(...)) at the moment.
    // How to support it?
}

fn gen_type_write(field: &syn::Field) -> TokenStream {
    match &field.ty {
        syn::Type::Path(type_path) => {
            if type_path.qself.is_some() {
                // Self qualifier?
                unimplemented!();
            }

            let path = &type_path.path;

            let name = &field.ident.as_ref().unwrap();

            if is_primitive(path) {
                let set_method = syn::Ident::new(&format!("set_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    writer.reborrow().#set_method(self.#name);
                };
            }

            if path.is_ident("String") || is_data(path) {
                let set_method = syn::Ident::new(&format!("set_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    writer.reborrow().#set_method(&self.#name);
                };
            }

            if let Some(inner_path) = get_list(path) {
                let init_method = syn::Ident::new(&format!("init_{}", &name), name.span());
                let list_write_iter = gen_list_write_iter(&inner_path);

                // In the cases of more complicated types, list_builder needs to be mutable.
                let let_list_builder =
                    if is_primitive(path) || path.is_ident("String") || is_data(path) {
                        quote! { let list_builder }
                    } else {
                        quote! { let mut list_builder }
                    };

                return quote_spanned! {field.span() =>
                    #let_list_builder = writer
                        .reborrow()
                        .#init_method(u32::try_from(self.#name.len()).unwrap());

                    for (index, item) in self.#name.iter().enumerate() {
                        #list_write_iter
                    }
                };
            }

            // Generic type:
            let init_method = syn::Ident::new(&format!("init_{}", &name), name.span());
            quote_spanned! {field.span() =>
                self.#name.write_capnp(&mut writer.reborrow().#init_method());
            }
        }
        _ => unimplemented!(),
    }
}

fn gen_list_read_iter(path: &syn::Path) -> TokenStream {
    if is_primitive(path) || path.is_ident("String") || is_data(path) {
        // A primitive list:
        quote! {
            res_vec.push(item_reader.into());
        }
    } else {
        // Not a primitive list:
        quote! {
            res_vec.push(#path::read_capnp(&item_reader)?);
        }
    }
    // TODO: It seems like we do not support List(List(...)) at the moment.
    // How to support it?
}

fn gen_type_read(field: &syn::Field) -> TokenStream {
    match &field.ty {
        syn::Type::Path(type_path) => {
            if type_path.qself.is_some() {
                // Self qualifier?
                unimplemented!();
            }

            let path = &type_path.path;

            let name = &field.ident.as_ref().unwrap();

            if is_primitive(path) {
                let get_method = syn::Ident::new(&format!("get_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    #name: reader.#get_method().into()
                };
            }

            if path.is_ident("String") || is_data(path) {
                let get_method = syn::Ident::new(&format!("get_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    #name: reader.#get_method()?.into()
                };
            }

            if let Some(inner_path) = get_list(path) {
                let get_method = syn::Ident::new(&format!("get_{}", &name), name.span());
                let list_read_iter = gen_list_read_iter(&inner_path);
                return quote_spanned! {field.span() =>
                    #name: {
                        let mut res_vec = Vec::new();
                        for item_reader in reader.#get_method()? {
                            // res_vec.push_back(read_named_relay_address(&named_relay_address)?);
                            #list_read_iter
                        }
                        res_vec
                    }
                };
            }

            // Generic type:
            let get_method = syn::Ident::new(&format!("get_{}", &name), name.span());
            quote_spanned! {field.span() =>
                #name: #type_path::read_capnp(&reader.#get_method()?)?
            }
        }
        _ => unimplemented!(),
    }
}

fn gen_write_capnp_named_struct(
    fields_named: &FieldsNamed,
    rust_struct: &Ident,
    capnp_struct: &Path,
) -> TokenStream {
    let recurse = fields_named
        .named
        .iter()
        .map(|field| gen_type_write(&field));

    quote! {
        impl<'a> WriteCapnp<'a> for #rust_struct {
            type WriterType = #capnp_struct::Builder<'a>;

            fn write_capnp(&'a self, writer: &'a mut Self::WriterType) {
                #(#recurse)*
            }
        }
    }
}

fn gen_read_capnp_named_struct(
    fields_named: &FieldsNamed,
    rust_struct: &Ident,
    capnp_struct: &Path,
) -> TokenStream {
    let recurse = fields_named.named.iter().map(|field| gen_type_read(field));

    quote! {
        impl<'a> ReadCapnp<'a> for #rust_struct {
            type ReaderType = #capnp_struct::Reader<'a>;

            fn read_capnp(reader: &'a Self::ReaderType) -> Result<Self, CapnpConvError> {
                Ok(#rust_struct {
                    #(#recurse,)*
                })
            }
        }
    }
}

fn gen_write_capnp_enum(
    _data_enum: &DataEnum,
    _rust_struct: &Ident,
    _capnp_struct: &Path,
) -> TokenStream {
    unimplemented!();
}

fn gen_read_capnp_enum(
    _data_enum: &DataEnum,
    _rust_struct: &Ident,
    _capnp_struct: &Path,
) -> TokenStream {
    unimplemented!();
}

/// Generate code for conversion between Rust and capnp structs.
#[proc_macro_attribute]
pub fn capnp_conv(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    // See: https://github.com/dtolnay/syn/issues/86
    // for information about arguments.

    // Name of capnp struct:
    let capnp_struct = parse_macro_input!(args as Path);
    let input = parse_macro_input!(input as DeriveInput);

    // Name of local struct:
    let rust_struct = &input.ident;

    let conversion = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields_named) => {
                // Example:
                // struct Point {
                //     x: u32,
                //     y: u32,
                // }
                let write_capnp =
                    gen_write_capnp_named_struct(fields_named, rust_struct, &capnp_struct);
                let read_capnp =
                    gen_read_capnp_named_struct(fields_named, rust_struct, &capnp_struct);

                quote! {
                    #write_capnp
                    #read_capnp
                }
            }
            Fields::Unnamed(_) | Fields::Unit => unimplemented!(),
        },
        Data::Enum(ref data_enum) => {
            // Example:
            // enum MyEnum {
            //     Type1(u32),
            //     Type2,
            //     Type3(MyStruct),
            // }
            let write_capnp = gen_write_capnp_enum(data_enum, rust_struct, &capnp_struct);
            let read_capnp = gen_read_capnp_enum(data_enum, rust_struct, &capnp_struct);

            quote! {
                #write_capnp
                #read_capnp
            }
        }
        Data::Union(_) => unimplemented!(),
    };

    let expanded = quote! {
        // Original structure
        #input
        // Generated mutual From conversion code:
        #conversion
    };

    proc_macro::TokenStream::from(expanded)
}
