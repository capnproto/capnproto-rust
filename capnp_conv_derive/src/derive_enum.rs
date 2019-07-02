use proc_macro2::TokenStream;
// use quote::{quote, quote_spanned};
// use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Index};
use syn::{DataEnum, Ident, Path};

pub fn gen_write_capnp_enum(
    _data_enum: &DataEnum,
    _rust_struct: &Ident,
    _capnp_struct: &Path,
) -> TokenStream {
    unimplemented!();
}

pub fn gen_read_capnp_enum(
    _data_enum: &DataEnum,
    _rust_struct: &Ident,
    _capnp_struct: &Path,
) -> TokenStream {
    unimplemented!();
}
