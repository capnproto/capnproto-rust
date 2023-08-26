use syn::{
    braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Brace,
    Ident, Token,
};

pub type CapnpLetStruct = CapnpAnonStruct<CapnpLetFieldPattern>;
pub type CapnpBuildStruct = CapnpAnonStruct<CapnpBuildFieldPattern>;

// capnp_let!(struct_pattern = subject)
pub struct CapnpLet {
    pub struct_pattern: CapnpLetStruct,
    pub equal_token: Token![=],
    pub ident: Ident,
}

// capnp_build!(person_builder, build_pattern)
pub struct CapnpBuild {
    pub subject: Ident,
    pub comma_token: Token![,],
    pub build_pattern: CapnpBuildStruct, // TODO Might be different for list
}

pub struct CapnpAnonStruct<FieldPattern: Parse> {
    pub brace_token: Brace,
    pub fields: Punctuated<FieldPattern, Token![,]>,
}

// pub struct CapnpField {
//     pub lhs: Ident,
//     pub colon_token: Option<Token![:]>,
//     pub rhs: Box<CapnpFieldPat>,
// }

// pub enum CapnpFieldPat {
//     AnonStruct(CapnpAnonStruct<CapnpLetFieldPattern>),
//     Ident(Ident),
// }

pub enum CapnpBuildFieldPattern {
    Name(Ident),
    ExpressionAssignment(Ident, syn::Expr),     // name = expr
    PatternAssignment(Ident, CapnpBuildStruct), // name : pat
    BuilderExtraction(Ident, Ident),            // name => name
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

impl Parse for CapnpBuild {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(CapnpBuild {
            subject: input.parse()?,
            comma_token: input.parse()?,
            build_pattern: input.parse()?,
        })
    }
}

impl<FieldPattern: Parse> Parse for CapnpAnonStruct<FieldPattern> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(CapnpAnonStruct {
            brace_token: braced!(content in input),
            //fields: syn::punctuated::Punctuated::parse_terminated(&content)?,
            fields: content.parse_terminated(FieldPattern::parse, Token![,])?,
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

impl Parse for CapnpBuildFieldPattern {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let res: Self;
        if input.peek(Token![,]) || input.is_empty() {
            res = Self::Name(name);
        } else if input.peek(Token![:]) {
            let _: Token![:] = input.parse()?;
            res = Self::PatternAssignment(name, input.parse()?);
        } else {
            let _: Token![=] = input.parse()?;
            if input.peek(Token![>]) {
                let _: Token![>] = input.parse()?;
                res = Self::BuilderExtraction(name, input.parse()?);
            } else {
                res = Self::ExpressionAssignment(name, input.parse()?);
            }
        }
        Ok(res)
    }
}

// impl Parse for CapnpField {
//     fn parse(input: ParseStream) -> syn::Result<Self> {
//         let lhs: Ident = input.parse()?;
//         let colon_token: Option<Token![:]> = input.parse()?;
//         let rhs: Box<CapnpFieldPat>;
//         if colon_token.is_none() {
//             // {.., lhs, ..}
//             rhs = Box::new(CapnpFieldPat::Ident(lhs.clone()));
//         } else if input.peek(Brace) {
//             // {.., lhs: {...}, ..}
//             rhs = Box::new(CapnpFieldPat::AnonStruct(CapnpAnonStruct::parse(input)?));
//         } else {
//             // {.., lhs: rhs, ..}
//             rhs = Box::new(CapnpFieldPat::Ident(input.parse()?));
//         }

//         Ok(CapnpField {
//             lhs,
//             colon_token,
//             rhs,
//         })
//     }
// }
