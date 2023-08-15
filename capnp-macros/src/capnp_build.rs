use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{punctuated::Punctuated, Ident, Token};

type Expr = TokenStream2;
type PatternStruct = dyn Iterator<Item = (Ident, Expr)>;
type PatternList = dyn Iterator<Item = Expr>;
// TODO Type check expr
// builder, {field => value}
fn assign_from_expression(builder: &Ident, field: Ident, expr: Expr) -> TokenStream2 {
    let field_setter = format_ident!("set_{}", field);
    quote! {
        #builder.#field_setter(#expr);
    }
}

// builder, {field => {pattern..}}
fn build_with_pattern<T: Iterator<Item = (Ident, Expr)>>(
    builder: Ident,
    field: Ident,
    pattern: T,
) -> TokenStream2 {
    let field_builder_name = format_ident!("{}_builder", field);
    let acquire_field_symbol = extract_symbol(builder, field, field_builder_name.clone());
    let assignments = pattern.map(|(field, expr)| {
        // TODO Needs to check whether expr is another pattern and behave accordingly
        assign_from_expression(&field_builder_name, field, expr)
    });
    quote! {
        #acquire_field_symbol
        #(#assignments)*
    }
}

// builder, {field: symbol_to_extract}
fn extract_symbol(builder: Ident, field: Ident, symbol_to_extract: Ident) -> TokenStream2 {
    let field_accessor = format_ident!("get_{}", field);
    quote! {
        let mut #symbol_to_extract = capnp_rpc::pry!(#builder.#field_accessor().into_result());
    }
}

// list_builder, [expr..]
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
        #( #list_builder.set(#enumerated); )*
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

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;

    #[test]
    fn test_assign_from_expression() {
        let builder = format_ident!("person_builder");
        let field = format_ident!("name");
        let expr = "John".into_token_stream();
        let lhs = assign_from_expression(&builder, field, expr);
        let rhs = quote! {
            person_builder.set_name("John");
        };
        assert_eq!(lhs.to_string(), rhs.to_string());
    }

    #[test]
    fn test_build_with_pattern() {
        let builder = format_ident!("person_builder");
        let field = format_ident!("birthdate");
        let field_value = vec![
            (format_ident!("day"), 1u8.into_token_stream()),
            (format_ident!("month"), 2u8.into_token_stream()),
            (format_ident!("year_as_text"), "1990".into_token_stream()),
        ]
        .into_iter();
        let lhs = build_with_pattern(builder, field, field_value);
        let rhs = quote! {
            let mut birthdate_builder = capnp_rpc::pry!(person_builder.get_birthdate().into_result());
            birthdate_builder.set_day(1u8);
            birthdate_builder.set_month(2u8);
            birthdate_builder.set_year_as_text("1990");
        };
        assert_eq!(lhs.to_string(), rhs.to_string());
    }

    #[test]
    fn test_build_list_with_elements() {
        let list_builder = format_ident!("list_builder");
        let mut elements = Punctuated::new();
        elements.push(quote!("a"));
        elements.push(quote!("b"));
        elements.push(quote!("c"));
        let lhs = build_list_with_elements(list_builder, elements);
        let rhs = quote! {
            list_builder.set(0usize, "a");
            list_builder.set(1usize, "b");
            list_builder.set(2usize, "c");
        };
        assert_eq!(lhs.to_string(), rhs.to_string());
    }

    #[test]
    fn test_build_list_with_patterns_struct() {
        let list_builder = format_ident!("list_builder");
        let struct_builder = format_ident!("person_builder");
        //let mut patterns = Punctuated::new();
        //patterns.push(())
        let lhs: TokenStream2 = todo!();
        //build_list_with_patterns_struct(list_builder, struct_builder, patterns.into_iter());
        let rhs = quote! {
            person_builder.set_name("a");
            list_builder.set(0usize, person_builder.clone());
            person_builder.set_name("b");
            list_builder.set(1usize, person_builder.clone());
            person_builder.set_name("c");
            list_builder.set(2usize, person_builder.clone());
        };
        assert_eq!(lhs.to_string(), rhs.to_string());
    }

    #[test]
    fn test_build_list_with_patterns_list() {
        todo!()
    }

    #[test]
    fn test_build_list_with_iterator() {
        let list_builder = format_ident!("list_builder");
        let day_builder = format_ident!("day_builder");
        let iterator = vec![1, 2, 3, 4, 5].into_iter();
        let f = |x: i32, y: &Ident| {
            assign_from_expression(y, format_ident!("day"), x.into_token_stream())
        };
        let lhs = build_list_with_iterator(list_builder, iterator, f, day_builder);
        let rhs = quote! {
            list_builder.set(0usize, "a");
            list_builder.set(1usize, "b");
            list_builder.set(2usize, "c");
        };
        assert_eq!(lhs.to_string(), rhs.to_string());
    }
}
