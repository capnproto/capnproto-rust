mod capnp_build;
mod capnp_let;
mod capnproto_rpc;
mod parse;
mod parse_capnp_build;
mod parse_capnp_let;

use crate::capnp_build::process_build_pry;
use crate::capnp_let::process_let_pry;
use crate::capnproto_rpc::process_capnproto_rpc;
use crate::parse_capnp_build::CapnpBuild;
use crate::parse_capnp_let::CapnpLet;
use proc_macro::TokenStream;

// capnp_let!({name, birthdate: {year, month, day}, email: contactEmail} = person)
/// Extracts fields from capnproto's struct readers.
///
/// # Usage
/// TODO: This section is unfinished
/// ```ignore
/// // Used within a function that returns Promise
/// capnp_let!({field1, field2 : name2, field3 : {inner_field: inner_name} } = struct_reader);
/// ```
/// Exposes `field1`, `name2`, `field3` and `inner_name` variables that correspond to appropriate fields from `struct_reader`.
#[proc_macro]
pub fn capnp_let(input: TokenStream) -> TokenStream {
    let CapnpLet {
        struct_pattern,
        ident,
        ..
    } = syn::parse_macro_input!(input as CapnpLet);
    let result = process_let_pry(struct_pattern, ident);
    result.into()
}

/// Assigns values to capnproto's struct and list builders.
///
/// # Usage
/// TODO: Those examples aren't comprehensive and need to be expanded
/// (For example with closure syntax)
/// ```ignore
/// // Used within a function that returns Promise
/// let field3 = "Example text 2";
/// capnp_build!(struct_builder, {
///     field1 = "Example text",
///     field2 : {
///         inner_field = 12
///     },
///     field3
/// };
/// ```
///
/// ```ignore
/// // Used within a function that returns Promise
/// capnp_build!(list_builder, [=1, =2, =3])
/// ```
#[proc_macro]
pub fn capnp_build(input: TokenStream) -> TokenStream {
    let CapnpBuild {
        subject,
        build_pattern,
        ..
    } = syn::parse_macro_input!(input as CapnpBuild);
    let result = process_build_pry(subject, build_pattern);
    result.into()
}

#[proc_macro_attribute]
pub fn capnproto_rpc(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as syn::ItemImpl);
    let result = process_capnproto_rpc(attr.into(), item);
    result.into()
}
