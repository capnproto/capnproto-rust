use syn::{
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Brace,
    Ident, Token,
};

// capnp_let!(struct_pattern = subject)
pub struct CapnpLet {
    pub struct_pattern: CapnpAnonStruct,
    pub equal_token: Token![=],
    pub ident: Ident,
}

// capnp_build!(person_builder, build_pattern)
pub struct CapnpBuild {
    pub subject: Ident,
    pub comma_token: Token![,],
    pub build_pattern: CapnpAnonStruct, // TODO Might be different for list
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

pub enum CapnpBuildFieldPattern {
    Name,
    ExpressionAssignment, // name = expr
    PatternAssignment,    // name : pat
    BuilderExtraction,    // name => name
}

pub enum CapnpLetFieldPattern {
    Name,             // name
    StructAssignment, // name: name
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

impl Parse for CapnpBuild {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(CapnpBuild {
            subject: input.parse()?,
            comma_token: input.parse()?,
            build_pattern: input.parse()?,
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
