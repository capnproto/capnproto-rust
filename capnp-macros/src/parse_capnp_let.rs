use crate::parse::*;
use syn::{
    parse::{Parse, ParseStream},
    token::Brace,
    Ident, Token,
};

pub type CapnpLetStruct = CapnpAnonStruct<CapnpLetFieldPattern>;

// capnp_let!(struct_pattern = subject)
pub struct CapnpLet {
    pub struct_pattern: CapnpLetStruct,
    pub equal_token: Token![=],
    pub ident: Ident,
}

pub enum CapnpLetFieldPattern {
    Name(Ident),                               // name
    ExtractToSymbol(Ident, Ident),             // name: name
    ExtractWithPattern(Ident, CapnpLetStruct), // name: struct_pattern
}

impl Parse for CapnpLet {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(CapnpLet {
            struct_pattern: input.parse()?,
            equal_token: input.parse()?,
            ident: input.parse()?,
        })
    }
}

impl Parse for CapnpLetFieldPattern {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let colon_token: Option<Token![:]> = input.parse()?;
        let res: Self;
        if colon_token.is_none() {
            res = Self::Name(name);
        } else if input.peek(Brace) {
            res = Self::ExtractWithPattern(name, input.parse()?);
        } else {
            let name2: Ident = input.parse()?;
            res = Self::ExtractToSymbol(name, name2);
        };
        Ok(res)
    }
}
