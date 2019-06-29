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

extern crate proc_macro;

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
// use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Index};
use syn::{parse_macro_input, Data, DataEnum, DeriveInput, Fields, FieldsNamed, Ident, Path};

fn is_primitive_type(ty: &syn::Type) -> bool {
    match ty {
        syn::Type::Path(type_path) => {
            // TODO: What is qself?
            if type_path.qself.is_some() {
                return false;
            }

            let path = &type_path.path;
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
        _ => false,
    }
}

fn gen_write_capnp_named_struct(
    fields_named: &FieldsNamed,
    rust_struct: &Ident,
    capnp_struct: &Path,
) -> TokenStream {
    let recurse = fields_named.named.iter().map(|f| {
        let name = &f.ident.as_ref().unwrap();
        // let init_method_str = format!("init_", ident);

        if is_primitive_type(&f.ty) {
            let set_method = syn::Ident::new(&format!("set_{}", &name), name.span());
            quote_spanned! {f.span() =>
                writer.reborrow().#set_method(self.#name);
            }
        } else {
            let init_method = syn::Ident::new(&format!("init_{}", &name), name.span());
            quote_spanned! {f.span() =>
                self.#name.write_capnp(&mut writer.reborrow().#init_method());
            }
        }
    });

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
    let recurse = fields_named.named.iter().map(|f| {
        let name = &f.ident.as_ref().unwrap();
        let ty = &f.ty;
        let get_method = syn::Ident::new(&format!("get_{}", &name), name.span());
        if is_primitive_type(ty) {
            quote_spanned! {f.span() =>
                #name: reader.#get_method()
            }
        } else {
            quote_spanned! {f.span() =>
                #name: #ty::read_capnp(&reader.#get_method()?)?
            }
        }
    });

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
