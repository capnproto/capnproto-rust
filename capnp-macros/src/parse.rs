use syn::{
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Brace,
    Ident, Token,
};

pub struct CapnpLet {
    pub anon_struct: CapnpAnonStruct,
    pub equal_token: Token![=],
    pub ident: Ident,
}

pub struct CapnpAnonStruct {
    pub brace_token: Brace,
    pub fields: Punctuated<CapnpField, Token![,]>,
}

pub struct CapnpField {
    pub lhs: Ident,
    pub colon_token: Option<Token![:]>,
    pub rhs: Box<CapnpFieldPat>,
}

pub enum CapnpFieldPat {
    AnonStruct(CapnpAnonStruct),
    Ident(Ident),
}

impl Parse for CapnpLet {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(CapnpLet {
            anon_struct: input.parse()?,
            equal_token: input.parse()?,
            ident: input.parse()?,
        })
    }
}

impl Parse for CapnpAnonStruct {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(CapnpAnonStruct {
            brace_token: braced!(content in input),
            fields: content.parse_terminated(CapnpField::parse, Token![,])?,
        })
    }
}

impl Parse for CapnpField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lhs: Ident = input.parse()?;
        let colon_token: Option<Token![:]> = input.parse()?;
        let rhs: Box<CapnpFieldPat>;
        if colon_token.is_none() {
            // {.., lhs, ..}
            rhs = Box::new(CapnpFieldPat::Ident(lhs.clone()));
        } else if input.peek(Brace) {
            // {.., lhs: {...}, ..}
            rhs = Box::new(CapnpFieldPat::AnonStruct(CapnpAnonStruct::parse(input)?));
        } else {
            // {.., lhs: rhs, ..}
            rhs = Box::new(CapnpFieldPat::Ident(input.parse()?));
        }

        Ok(CapnpField {
            lhs,
            colon_token,
            rhs,
        })
    }
}
