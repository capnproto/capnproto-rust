use crate::parse::CapnpAnonStruct;
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{Brace, Bracket},
    Ident, Token,
};

pub type CapnpBuildStruct = CapnpAnonStruct<CapnpBuildFieldPattern>;

// capnp_build!(person_builder, build_pattern)
pub struct CapnpBuild {
    pub subject: Ident,
    pub comma_token: Token![,],
    pub build_pattern: CapnpBuildPattern,
}

pub enum CapnpBuildPattern {
    StructPattern(CapnpBuildStruct), // {...}
    ListPattern(ListPattern),        // [...]
}

pub enum CapnpBuildFieldPattern {
    Name(Ident),                                 // name
    ExpressionAssignment(Ident, syn::Expr),      // name = expr
    PatternAssignment(Ident, CapnpBuildPattern), // name : pat
    BuilderExtraction(Ident, syn::ExprClosure),  // name => closure
}

pub enum ListPattern {
    ListComprehension(syn::ExprForLoop), // for RustPattern in IteratorExpression {BlockExpression}
    ListElements(Punctuated<ListElementPattern, Token![,]>), // [= 13, [...], {...}]
}

pub enum ListElementPattern {
    SimpleExpression(syn::Expr),     // = value
    StructPattern(CapnpBuildStruct), // {...}
    ListPattern(ListPattern),        // [...]
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

impl Parse for CapnpBuildPattern {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let res = if input.peek(Brace) {
            Self::StructPattern(CapnpBuildStruct::parse(input)?)
        } else {
            Self::ListPattern(ListPattern::parse(input)?)
        };
        Ok(res)
    }
}

impl Parse for CapnpBuildFieldPattern {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let res = if input.peek(Token![,]) || input.is_empty() {
            // name
            Self::Name(name)
        } else if input.peek(Token![:]) {
            // name : pattern
            let _: Token![:] = input.parse()?;
            Self::PatternAssignment(name, input.parse()?)
        } else {
            let _: Token![=] = input.parse()?;
            if input.peek(Token![>]) {
                // name => closure
                let _: Token![>] = input.parse()?;
                Self::BuilderExtraction(name, input.parse()?)
            } else {
                // name = value
                Self::ExpressionAssignment(name, input.parse()?)
            }
        };
        Ok(res)
    }
}

impl Parse for ListPattern {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        bracketed!(content in input);
        let res: Self = if content.peek(Token![for]) {
            Self::ListComprehension(content.parse()?)
        } else {
            Self::ListElements(content.parse_terminated(ListElementPattern::parse, Token![,])?)
        };
        Ok(res)
    }
}

impl Parse for ListElementPattern {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let res: Self = if input.peek(Token![=]) {
            let _: Token![=] = input.parse()?;
            Self::SimpleExpression(input.parse()?)
        } else if input.peek(Bracket) {
            Self::ListPattern(input.parse()?)
        } else {
            Self::StructPattern(input.parse()?)
        };
        Ok(res)
    }
}
