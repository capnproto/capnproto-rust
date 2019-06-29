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
        syn::punctuated::Pair::End(last_ident) => last_ident,
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

    if !arg_ty_path.path.is_ident("u8") {
        return false;
    }

    true
}

/// Check if the path represents a Vec<SomeStruct>, where SomeStruct != u8
fn is_list(path: &syn::Path) -> bool {
    if !path.is_ident("Vec") {
        return false;
    }
    // Could be a List, or Data.
    // If this is Vec<u8>, we decide that it is Data. Otherwise we decide it is a List.
    //
    let last_ident = match path.segments.last().unwrap() {
        syn::punctuated::Pair::End(last_ident) => last_ident,
        _ => unreachable!(),
    };

    let angle = match &last_ident.arguments {
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

    if arg_ty_path.path.is_ident("u8") {
        return false;
    }

    true
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

            if path.is_ident("String") {
                let set_method = syn::Ident::new(&format!("set_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    writer.reborrow().#set_method(&self.#name);
                };
            }

            if is_data(path) {
                let set_method = syn::Ident::new(&format!("set_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    writer.reborrow().#set_method(&self.#name);
                };
            }

            if is_list(path) {
                // List case:
                unimplemented!();
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
                    #name: reader.#get_method()
                };
            }

            if path.is_ident("String") {
                let get_method = syn::Ident::new(&format!("get_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    #name: reader.#get_method()?.to_string()
                };
            }

            if is_data(path) {
                let get_method = syn::Ident::new(&format!("get_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    #name: reader.#get_method()?.to_vec()
                };
            }

            if is_list(path) {
                // List case:
                unimplemented!();
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
