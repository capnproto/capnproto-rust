use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::Ident;

use crate::parse_capnp_build::{
    CapnpBuildFieldPattern, CapnpBuildPattern, CapnpBuildStruct, ListElementPattern, ListPattern,
};

pub fn process_build_pry(builder: Ident, build_pattern: CapnpBuildPattern) -> TokenStream2 {
    match build_pattern {
        // capnp_build!(struct_builder, {..});
        CapnpBuildPattern::StructPattern(struct_pattern) => {
            process_struct_pattern(&builder, struct_pattern)
        }
        // capnp_build!(list_builder, [..]);
        CapnpBuildPattern::ListPattern(list_pattern) => {
            process_list_pattern(&builder, list_pattern)
        }
    }
}

// capnp_build!(struct_builder, {
//     name1,
//     name2 = expr1,
//     name3: {...},
//     name4: [...],
//     name5 => |...|{...}
// })
fn process_struct_pattern(builder: &Ident, struct_pattern: CapnpBuildStruct) -> TokenStream2 {
    let mut res = TokenStream2::new();
    for field_pattern in struct_pattern.fields.into_iter() {
        let to_append = match field_pattern {
            // name
            CapnpBuildFieldPattern::Name(name) => {
                let field_setter = format_ident!("set_{}", name);
                quote!(#builder.reborrow().#field_setter((#name).into());)
            }
            // name = expr
            CapnpBuildFieldPattern::ExpressionAssignment(name, expr) => {
                let field_setter = format_ident!("set_{}", name);
                quote!(#builder.reborrow().#field_setter((#expr).into());)
            }
            // name : {...}
            CapnpBuildFieldPattern::PatternAssignment(
                name,
                CapnpBuildPattern::StructPattern(struct_pattern),
            ) => {
                let struct_builder = format_ident!("{}_builder", name);
                let field_accessor = format_ident!("get_{}", name);
                let head = quote! (
                    let mut #struct_builder = (::capnp_rpc::pry!(#builder.reborrow().#field_accessor()));
                );
                let tail = process_struct_pattern(&struct_builder, struct_pattern);
                quote!(#head #tail)
            }
            // name : [...]
            CapnpBuildFieldPattern::PatternAssignment(
                name,
                CapnpBuildPattern::ListPattern(list_pattern),
            ) => {
                let list_builder = format_ident!("{}_builder", name);
                let list_initializer = format_ident!("init_{}", name);
                let length = get_list_size(&list_pattern);
                let head = quote!(
                    let mut #list_builder = #builder.reborrow().#list_initializer(#length as u32);
                );
                let tail = process_list_pattern(&list_builder, list_pattern);
                quote!(#head #tail)
            }
            // name => |...|{...}
            CapnpBuildFieldPattern::BuilderExtraction(name, closure) => {
                let field_accessor = format_ident!("get_{}", name);
                quote! {
                    (#closure)(::capnp_rpc::pry!(::capnp::IntoResult::into_result(#builder.reborrow().#field_accessor())));
                }
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

            // Extract (item_builder, pattern) as an argument
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
                let to_append = match item {
                    ListElementPattern::SimpleExpression(expr) => {
                        // TODO Doesn't support structs from value, as in =some_struct_builder_expr
                        quote!(#builder.reborrow().set(#i as u32, (#expr).into());)
                    }
                    ListElementPattern::StructPattern(struct_pattern) => {
                        let struct_builder = format_ident!("{}_builder_{}", builder, i);
                        let head =
                            quote!(let mut #struct_builder = #builder.reborrow().get(#i as u32););
                        let tail = process_struct_pattern(&struct_builder, struct_pattern);
                        quote!(#head #tail)
                    }
                    ListElementPattern::ListPattern(list_pattern) => {
                        let list_builder = format_ident!("{}_builder_{}", builder, i);
                        let length = get_list_size(&list_pattern);
                        let head = quote!(let mut #list_builder = #builder.reborrow().init(#i as u32, #length as u32););
                        let tail = process_list_pattern(&list_builder, list_pattern);
                        quote!(#head #tail)
                    }
                };
                res.extend(to_append);
            }
            res
        }
    }
}

fn get_list_size(list_pattern: &ListPattern) -> TokenStream2 {
    match list_pattern {
        ListPattern::ListComprehension(for_expr) => {
            let it = &for_expr.expr;
            quote!(#it.len())
        }
        ListPattern::ListElements(elems) => {
            let res = elems.len();
            quote!(#res)
        }
    }
}
