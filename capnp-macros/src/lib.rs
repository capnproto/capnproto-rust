mod parse;

use crate::parse::{CapnpAnonStruct, CapnpField, CapnpFieldPat, CapnpLet};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::Ident;

// capnp_let!({name, birthdate: {year, month, day}, email: contactEmail} = person)
#[proc_macro]
pub fn capnp_let(input: TokenStream) -> TokenStream {
    let CapnpLet {
        anon_struct, ident, ..
    } = syn::parse_macro_input!(input as CapnpLet);
    let result = process_inner_pry(anon_struct, ident).unwrap();
    result.into()
}

/// Takes `expr` as an identifier of a capnproto Reader type of some struct and extracts fields specified in `pat`.
/// `pat` is of the form `{capnpfield1, capnpfield2, ...}`. Each `capnpfield` is a pair `lhs: rhs`.
/// Returns token stream of assignments for variables specified recursively in `rhs`.
fn process_inner_pry(pat: CapnpAnonStruct, expr: Ident) -> syn::Result<TokenStream2> {
    let mut res = TokenStream2::new();
    for field in pat.fields.into_iter() {
        let CapnpField { lhs, rhs, .. } = field;

        let field_accessor = format_ident!("get_{}", lhs);
        let to_append = match *rhs {
            CapnpFieldPat::Ident(ident) => {
                quote!(let #ident = #expr.reborrow().#field_accessor();)
            }
            CapnpFieldPat::AnonStruct(s) => {
                let head = quote!(let #lhs = capnp_rpc::pry!(#expr.reborrow().#field_accessor()););
                let tail = process_inner_pry(s, lhs)?;
                quote!(#head #tail)
            }
        };
        //dbg!(&to_append.to_string());
        res.extend(to_append);
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() -> syn::Result<()> {
        let input = quote! {
            {name, birthdate: {year_as_text: year, month, day}, email: contactEmail} = person
        }; // person is person_capnp::person::Reader
        let CapnpLet {
            anon_struct, ident, ..
        } = syn::parse2(input)?;
        process_inner_pry(anon_struct, ident)?;
        Ok(())
    }
}
