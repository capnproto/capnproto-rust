use capnp::traits::OwnedStruct;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{punctuated::Punctuated, Ident, Token};

use crate::parse::{
    CapnpBuildFieldPattern, CapnpBuildPattern, CapnpBuildStruct, ListElementPattern, ListPattern,
};

pub fn process_build_pry(builder: Ident, build_pattern: CapnpBuildPattern) -> TokenStream2 {
    match build_pattern {
        CapnpBuildPattern::StructPattern(struct_pattern) => {
            process_struct_pattern(&builder, struct_pattern)
        }
        CapnpBuildPattern::ListPattern(list_pattern) => {
            process_list_pattern(&builder, list_pattern)
        }
    }
}

fn process_struct_pattern(builder: &Ident, struct_pattern: CapnpBuildStruct) -> TokenStream2 {
    let mut res = TokenStream2::new();
    for field_pattern in struct_pattern.fields.into_iter() {
        let to_append = match field_pattern {
            // name
            CapnpBuildFieldPattern::Name(name) => {
                let x: syn::ExprPath = syn::parse2(name.to_token_stream()).unwrap(); //We know name is an Ident and therefore a Path
                let x: syn::Expr = syn::Expr::Path(x);
                assign_from_expression(&builder, &name, &x)
            }
            // name = expr
            CapnpBuildFieldPattern::ExpressionAssignment(name, expr) => {
                assign_from_expression(&builder, &name, &expr)
            }
            // name : {...}
            CapnpBuildFieldPattern::PatternAssignment(
                name,
                CapnpBuildPattern::StructPattern(struct_pattern),
            ) => {
                let struct_builder = format_ident!("{}_builder", name);
                let temp = extract_symbol(&builder, &name, &struct_builder);
                let temp2 = process_struct_pattern(&struct_builder, struct_pattern);
                quote! {
                    #temp
                    #temp2
                }
            }
            // name : [...]
            CapnpBuildFieldPattern::PatternAssignment(
                name,
                CapnpBuildPattern::ListPattern(list_pattern),
            ) => {
                let list_builder = format_ident!("{}_builder", name);
                let list_initializer = format_ident!("init_{}", name);
                let length = get_list_size(&list_pattern);
                let temp = quote!(let mut #list_builder = #builder.reborrow().#list_initializer(#length as u32););
                let temp2 = process_list_pattern(&list_builder, list_pattern);
                quote! {
                    #temp
                    #temp2
                }
            }
            // name => |...|{...}
            CapnpBuildFieldPattern::BuilderExtraction(name, closure) => {
                extract_symbol_new(&builder, &name, &closure)
            }
        };
        res.extend(to_append);
    }
    res
}

fn process_list_pattern(builder: &Ident, list_pattern: ListPattern) -> TokenStream2 {
    match list_pattern {
        // [for x in y {...}]
        ListPattern::ListComprehension(for_expr) => {
            let syn::ExprForLoop {
                pat, expr, body, ..
            } = for_expr;
            let syn::Pat::Tuple(t) = *pat else {
                panic!("Argument for capnp_build's list comprehension requires a tuple (item_builder, contents)");
            };
            if t.elems.len() != 2 {
                panic!(
                    "Argument for capnp_build's list comprehension requires has to have 2 elements"
                );
            }
            let (item_builder, pattern_part) = (t.elems.first().unwrap(), t.elems.last().unwrap());
            let index_name = format_ident!("{}_listcomprehension_index", builder);
            quote! {
                for (#index_name, #pattern_part) in #expr.enumerate() {
                    let mut #item_builder = #builder.reborrow().get(#index_name as u32);
                    #body;
                }
            }
        }
        // [=a, =b, [...], {...}]
        ListPattern::ListElements(elems) => {
            let mut res = TokenStream2::new();
            for (i, item) in elems.into_iter().enumerate() {
                let i = i as u32;
                let to_append = match item {
                    ListElementPattern::SimpleExpression(expr) => {
                        // TODO Doesn't support structs from value, as in =some_struct_builder_expr
                        quote! {
                            #builder.reborrow().set(#i, (#expr).into());
                        }
                    }
                    ListElementPattern::StructPattern(struct_pattern) => {
                        let struct_builder = format_ident!("{}_builder_{}", builder, i);
                        let temp = quote!(let mut #struct_builder = #builder.reborrow().get(#i););
                        let temp2 = process_struct_pattern(&struct_builder, struct_pattern);
                        quote! {
                            #temp
                            #temp2
                        }
                    }
                    ListElementPattern::ListPattern(list_pattern) => {
                        let list_builder = format_ident!("{}_builder_{}", builder, i);
                        let length = get_list_size(&list_pattern);
                        let temp =
                            quote!(let mut #list_builder = #builder.reborrow().init(#i, #length););
                        let temp2 = process_list_pattern(&list_builder, list_pattern);
                        quote! {
                            #temp
                            #temp2
                        }
                    }
                };
                res.extend(to_append);
            }
            res
        }
    }
}

// builder, {field = value}
fn assign_from_expression(builder: &Ident, field: &Ident, expr: &syn::Expr) -> TokenStream2 {
    let field_setter = format_ident!("set_{}", field);
    quote! {
        #builder.reborrow().#field_setter((#expr).into());
    }
}

// builder, {field : {pattern..}}
// fn build_with_pattern<T: Iterator<Item = CapnpBuildFieldPattern>>(
//     builder: &Ident,
//     field: &Ident,
//     pattern: T,
// ) -> TokenStream2 {
//     let field_builder_name = format_ident!("{}_builder", field);
//     let acquire_field_symbol = extract_symbol(&builder, &field, &field_builder_name);
//     let assignments = pattern
//         .map(|inner_field| process_struct_field_pattern(&field_builder_name, inner_field).unwrap());
//     quote! {
//         #acquire_field_symbol
//         #(#assignments)*
//     }
// }

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
    elements: Punctuated<syn::Expr, Token![,]>,
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

fn build_list_with_patterns_struct<T: Iterator<Item = (Ident, syn::Expr)>>(
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

fn get_list_size(list_pattern: &ListPattern) -> TokenStream2 {
    match list_pattern {
        ListPattern::ListComprehension(for_expr) => {
            let it = &for_expr.expr;
            quote! {
                #it.len()
            }
        }
        ListPattern::ListElements(elems) => {
            let res = elems.len();
            quote!(#res)
        }
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

// Legacy list code
//  CapnpBuildPattern::ListPattern(list_pattern) => match list_pattern {
//     crate::parse::ListPattern::ListComprehension(for_expr) => todo!(),
//     crate::parse::ListPattern::ListElements(elements) => {
//         let enumerated: Vec<(usize, ListElementPattern)> =
//             elements.into_iter().enumerate().collect();
//         let len = enumerated.len();
//         //let initializer = format_ident!("init_{}", &builder);
//         //res.extend(quote!(#.#initializer(#len)));
//         for (idx, field_pat) in enumerated {
//             let to_append = match field_pat {
//                 crate::parse::ListElementPattern::SimpleExpression(expr) => {
//                     quote!(#builder.set(#idx, #expr); )
//                 }
//                 crate::parse::ListElementPattern::StructPattern(struct_pattern) => {
//                     let mut struct_res = TokenStream2::new();
//                     let struct_name: Ident = format_ident!("_struct_{}", &builder);
//                     struct_res.extend(quote!(let mut #struct_name = #builder.reborrow().get(#idx as u32);));
//                     // TODO Create struct builder with a given name first
//                     let to_append = process_struct_pattern(&struct_name, struct_pattern);
//                     struct_res.extend(to_append);
//                     struct_res
//                 }
//                 crate::parse::ListElementPattern::ListPattern(list_pattern) => todo!(),
//             };
//             res.extend(to_append);
//         }
//     }
// },
// }
//Ok(res)
