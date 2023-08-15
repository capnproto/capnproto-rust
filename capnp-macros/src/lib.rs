mod capnp_build;
mod capnp_let;
mod parse;

use crate::capnp_let::process_let_pry;
use crate::parse::CapnpLet;
use proc_macro::TokenStream;
use syn::parse_macro_input;

// capnp_let!({name, birthdate: {year, month, day}, email: contactEmail} = person)
#[proc_macro]
pub fn capnp_let(input: TokenStream) -> TokenStream {
    let CapnpLet {
        anon_struct, ident, ..
    } = parse_macro_input!(input as CapnpLet);
    let result = process_let_pry(anon_struct, ident).unwrap();
    result.into()
}

#[proc_macro]
pub fn capnp_build(input: TokenStream) -> TokenStream {
    todo!()
    //     let CapnpLet {
    //         anon_struct,
    //         ident: expr,
    //         ..
    //     } = syn::parse_macro_input!(input as CapnpLet);
    //     let result = process_build(anon_struct, expr).unwrap();
    //     result.into()
}

// fn process_build(pat: CapnpAnonStruct, expr: Ident) -> syn::Result<TokenStream2> {
//     let mut res = TokenStream2::new();
//     for field in pat.fields.into_iter() {
//         let CapnpField { lhs, rhs, .. } = field;
//         let field_setter = format_ident!("set_{}", lhs);
//         let to_append = match *rhs {
//             CapnpFieldPat::Ident(inner_expr) => {
//                 quote! {
//                     #expr.#field_setter(#inner_expr)
//                 }
//             }
//             CapnpFieldPat::AnonStruct(s) => {
//                 let expr_builder = format_ident!("{}_builder", expr);
//                 let head = quote! {
//                     let #expr_builder = #expr.init_
//                 };
//             }
//         };
//     }
//     Ok(res)
// }
