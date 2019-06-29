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
use syn::{parse_macro_input, Data, DataEnum, DeriveInput, Fields, FieldsNamed, Ident};

fn gen_into_capnp_named_struct(
    fields_named: &FieldsNamed,
    rust_struct: &Ident,
    capnp_struct: &Ident,
) -> TokenStream {
    // let capnp_write = unimplemented!();

    let recurse = fields_named.named.iter().map(|f| {
        let name = &f.ident;
        quote_spanned! {f.span() =>
            self.#name.write_capnp(&mut writer.reborrow().init_#name());
        }
    });

    quote! {
        impl CapnpWriter for #rust_struct {
            type WriterType = #capnp_struct::Writer;

            fn write_capnp(self, writer: &mut Self::WriterType) {
                #(#recurse)*
            }
        }
    }
}

fn gen_from_capnp_named_struct(
    fields_named: &FieldsNamed,
    rust_struct: &Ident,
    capnp_struct: &Ident,
) -> TokenStream {
    let recurse = fields_named.named.iter().map(|f| {
        let name = &f.ident;
        quote_spanned! {f.span() =>
            // route: deser_friends_route(&route_capacity_rate_reader.get_route()?)?,
            #name: TODO_TYPE_NAME::read_capnp(&reader.get_#name()?)?
        }
    });

    quote! {
        impl CapnpReader for #rust_struct {
            type ReaderType = #capnp_struct::Reader;

            fn read_capnp(reader: &mut Self::ReaderType) {
                Ok(#rust_struct {
                    #(#recurse,)*
                })
            }
        }
    }
}

fn gen_into_capnp_enum(
    _data_enum: &DataEnum,
    _rust_struct: &Ident,
    _capnp_struct: &Ident,
) -> TokenStream {
    unimplemented!();
}

fn gen_from_capnp_enum(
    _data_enum: &DataEnum,
    _rust_struct: &Ident,
    _capnp_struct: &Ident,
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
    let capnp_struct = parse_macro_input!(args as Ident);
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
                let into_capnp =
                    gen_into_capnp_named_struct(fields_named, rust_struct, &capnp_struct);
                let from_capnp =
                    gen_from_capnp_named_struct(fields_named, rust_struct, &capnp_struct);

                quote! {
                    #into_capnp
                    #from_capnp
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
            let into_capnp = gen_into_capnp_enum(data_enum, rust_struct, &capnp_struct);
            let from_capnp = gen_from_capnp_enum(data_enum, rust_struct, &capnp_struct);

            quote! {
                #into_capnp
                #from_capnp
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
