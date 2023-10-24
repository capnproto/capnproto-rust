use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{punctuated::Punctuated, Ident, Token};

use crate::parse::{CapnpBuildFieldPattern, CapnpBuildPattern, ListElementPattern};

type Expr = TokenStream2;
type PatternStruct = dyn Iterator<Item = (Ident, Expr)>;
type PatternList = dyn Iterator<Item = Expr>;

pub fn process_build_pry(
    subject: Ident,
    build_pattern: CapnpBuildPattern,
) -> syn::Result<TokenStream2> {
    let mut res = TokenStream2::new();
    match build_pattern {
        CapnpBuildPattern::StructPattern(struct_pattern) => {
            for field_pat in struct_pattern.fields.into_iter() {
                let to_append = process_struct_field_pattern(&subject, field_pat);
                res.extend(to_append);
            }
        }
        CapnpBuildPattern::ListPattern(list_pattern) => match list_pattern {
            crate::parse::ListPattern::ListComprehension => todo!(),
            crate::parse::ListPattern::ListElements(elements) => {
                let enumerated: Vec<(usize, ListElementPattern)> =
                    elements.into_iter().enumerate().collect();
                let len = enumerated.len();
                //let initializer = format_ident!("init_{}", &subject);
                //res.extend(quote!(#.#initializer(#len)));
                for (idx, field_pat) in enumerated {
                    let to_append = match field_pat {
                        crate::parse::ListElementPattern::SimpleExpression(expr) => {
                            quote!(#subject.set(#idx, #expr); )
                        }
                        crate::parse::ListElementPattern::StructPattern(struct_pattern) => {
                            let mut struct_res = TokenStream2::new();
                            let struct_name: Ident = format_ident!("_struct_{}", &subject);
                            struct_res.extend(quote!(let mut #struct_name = #subject.reborrow().get(#idx as u32);));
                            // TODO Create struct builder with a given name first
                            for field_pat in struct_pattern.fields.into_iter() {
                                let to_append =
                                    process_struct_field_pattern(&struct_name, field_pat);
                                struct_res.extend(to_append);
                            }
                            struct_res
                        }
                        crate::parse::ListElementPattern::ListPattern(list_pattern) => todo!(),
                    };
                    res.extend(to_append);
                }
            }
        },
    }
    Ok(res)
}

fn process_struct_field_pattern(
    subject: &Ident,
    field_pat: CapnpBuildFieldPattern,
) -> syn::Result<TokenStream2> {
    let res = match field_pat {
        CapnpBuildFieldPattern::Name(name) => {
            let x: syn::ExprPath = syn::parse2(name.to_token_stream())?;
            let x: syn::Expr = syn::Expr::Path(x);
            assign_from_expression(&subject, &name, &x)
        }
        CapnpBuildFieldPattern::ExpressionAssignment(name, expr) => {
            assign_from_expression(&subject, &name, &expr)
        }
        CapnpBuildFieldPattern::PatternAssignment(name, pat) => {
            build_with_pattern(&subject, &name, pat.fields.into_iter())
        }
        CapnpBuildFieldPattern::BuilderExtraction(name1, closure) => {
            extract_symbol_new(&subject, &name1, &closure)
        }
    };
    Ok(res)
}

// builder, {field = value}
fn assign_from_expression(builder: &Ident, field: &Ident, expr: &syn::Expr) -> TokenStream2 {
    let field_setter = format_ident!("set_{}", field);
    quote! {
        #builder.reborrow().#field_setter((#expr).into());
    }
}

// builder, {field : {pattern..}}
fn build_with_pattern<T: Iterator<Item = CapnpBuildFieldPattern>>(
    builder: &Ident,
    field: &Ident,
    pattern: T,
) -> TokenStream2 {
    let field_builder_name = format_ident!("{}_builder", field);
    let acquire_field_symbol = extract_symbol(&builder, &field, &field_builder_name);
    let assignments = pattern
        .map(|inner_field| process_struct_field_pattern(&field_builder_name, inner_field).unwrap());
    quote! {
        #acquire_field_symbol
        #(#assignments)*
    }
}

// builder, {field => symbol_to_extract}
fn extract_symbol(builder: &Ident, field: &Ident, symbol_to_extract: &Ident) -> TokenStream2 {
    let field_accessor = format_ident!("get_{}", field);
    quote! {
        let mut #symbol_to_extract = (capnp_rpc::pry!(#builder.reborrow().#field_accessor().into_result()));
    }
}

fn extract_symbol_new(
    builder: &Ident,
    field: &Ident,
    closure_to_execute: &syn::ExprClosure,
) -> TokenStream2 {
    let field_accessor = format_ident!("get_{}", field);
    quote! {
        (#closure_to_execute)(capnp_rpc::pry!(#builder.reborrow().#field_accessor().into_result()));
    }
}

// list_builder, [=expr..]
fn build_list_with_elements(
    list_builder: Ident,
    elements: Punctuated<Expr, Token![,]>,
) -> TokenStream2 {
    let enumerated: Vec<TokenStream2> = elements
        .into_iter()
        .enumerate()
        .map(|(idx, expr)| quote!(#idx, #expr))
        .collect();
    quote! {
        #( #list_builder.set_with_caveats(#enumerated); )*
    }
}

fn build_list_with_patterns_struct<T: Iterator<Item = (Ident, Expr)>>(
    list_builder: Ident,
    struct_builder: Ident,
    pattern_struct: T,
) -> TokenStream2 {
    //build_with_pattern(struct_builder, field, pattern)
    //pattern_struct.map(|(field, expr) |)
    //quote! {
    //    #(
    //        #list_builder.set(#enumerated); )*
    //}
    todo!()
}

fn build_list_with_iterator<T, F>(
    list_builder: Ident,
    iter: T,
    f: F,
    struct_builder: Ident,
) -> TokenStream2
where
    T: ExactSizeIterator,
    F: Fn(T::Item, &Ident) -> TokenStream2,
{
    let structs = iter
        .map(|x| f(x, &struct_builder))
        .enumerate()
        .map(|(idx, expr)| quote!(#idx, #expr));
    quote! {
        #( #list_builder.set(#structs); )*
    }
}

// TODO Tests with old syntax
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use quote::ToTokens;

//     #[test]
//     fn test_assign_from_expression() {
//         let builder = format_ident!("person_builder");
//         let field = format_ident!("name");
//         let expr = "John".into_token_stream();
//         let lhs = assign_from_expression(&builder, &field, expr);
//         let rhs = quote! {
//             person_builder.set_name("John");
//         };
//         assert_eq!(lhs.to_string(), rhs.to_string());
//     }

//     #[test]
//     fn test_build_with_pattern() {
//         let builder = format_ident!("person_builder");
//         let field = format_ident!("birthdate");
//         let field_value = vec![
//             (format_ident!("day"), 1u8.into_token_stream()),
//             (format_ident!("month"), 2u8.into_token_stream()),
//             (format_ident!("year_as_text"), "1990".into_token_stream()),
//         ]
//         .into_iter();
//         let lhs = build_with_pattern(&builder, &field, field_value);
//         let rhs = quote! {
//             let mut birthdate_builder = capnp_rpc::pry!(person_builder.get_birthdate().into_result());
//             birthdate_builder.set_day(1u8);
//             birthdate_builder.set_month(2u8);
//             birthdate_builder.set_year_as_text("1990");
//         };
//         assert_eq!(lhs.to_string(), rhs.to_string());
//     }

//     #[test]
//     fn test_build_list_with_elements() {
//         let list_builder = format_ident!("list_builder");
//         let mut elements = Punctuated::new();
//         elements.push(quote!("a"));
//         elements.push(quote!("b"));
//         elements.push(quote!("c"));
//         let lhs = build_list_with_elements(list_builder, elements);
//         let rhs = quote! {
//             list_builder.set(0usize, "a");
//             list_builder.set(1usize, "b");
//             list_builder.set(2usize, "c");
//         };
//         assert_eq!(lhs.to_string(), rhs.to_string());
//     }

//     #[test]
//     fn test_build_list_with_patterns_struct() {
//         let list_builder = format_ident!("list_builder");
//         let struct_builder = format_ident!("person_builder");
//         //let mut patterns = Punctuated::new();
//         //patterns.push(())
//         let lhs: TokenStream2 = todo!();
//         //build_list_with_patterns_struct(list_builder, struct_builder, patterns.into_iter());
//         let rhs = quote! {
//             person_builder.set_name("a");
//             list_builder.set(0usize, person_builder.clone());
//             person_builder.set_name("b");
//             list_builder.set(1usize, person_builder.clone());
//             person_builder.set_name("c");
//             list_builder.set(2usize, person_builder.clone());
//         };
//         assert_eq!(lhs.to_string(), rhs.to_string());
//     }

//     #[test]
//     fn test_build_list_with_patterns_list() {
//         todo!()
//     }

//     #[test]
//     fn test_build_list_with_iterator() {
//         let list_builder = format_ident!("list_builder");
//         let day_builder = format_ident!("day_builder");
//         let iterator = vec![1, 2, 3, 4, 5].into_iter();
//         let f = |x: i32, y: &Ident| {
//             assign_from_expression(y, &format_ident!("day"), x.into_token_stream())
//         };
//         let lhs = build_list_with_iterator(list_builder, iterator, f, day_builder);
//         let rhs = quote! {
//             list_builder.set(0usize, "a");
//             list_builder.set(1usize, "b");
//             list_builder.set(2usize, "c");
//         };
//         assert_eq!(lhs.to_string(), rhs.to_string());
//     }
// }
