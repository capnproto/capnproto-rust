// use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Index};

/// Is a primitive type?
pub fn is_primitive(path: &syn::Path) -> bool {
    path.is_ident("u8")
        || path.is_ident("u16")
        || path.is_ident("u32")
        || path.is_ident("u64")
        || path.is_ident("i8")
        || path.is_ident("i16")
        || path.is_ident("i32")
        || path.is_ident("i64")
        || path.is_ident("bool")
}

/// Check if the path represents a Vec<u8>
pub fn is_data(path: &syn::Path) -> bool {
    let last_segment = match path.segments.last().unwrap() {
        syn::punctuated::Pair::End(last_segment) => last_segment,
        _ => unreachable!(),
    };
    if &last_segment.ident.to_string() != "Vec" {
        return false;
    }
    let angle = match &last_segment.arguments {
        syn::PathArguments::AngleBracketed(angle) => {
            if angle.args.len() > 1 {
                unreachable!("Too many arguments for Vec!");
            }
            angle
        }
        _ => unreachable!("Vec with arguments that are not angle bracketed!"),
    };
    let last_arg = match angle.args.last().unwrap() {
        syn::punctuated::Pair::End(last_arg) => last_arg,
        _ => return false,
    };

    let arg_ty = match last_arg {
        syn::GenericArgument::Type(arg_ty) => arg_ty,
        _ => return false,
    };

    let arg_ty_path = match arg_ty {
        syn::Type::Path(arg_ty_path) => arg_ty_path,
        _ => return false,
    };

    if arg_ty_path.qself.is_some() {
        return false;
    }

    if !arg_ty_path.path.is_ident("u8") {
        return false;
    }

    true
}

/// Check if the path represents a Vec<SomeStruct>, where SomeStruct != u8
pub fn get_list(path: &syn::Path) -> Option<syn::Path> {
    let last_segment = match path.segments.last().unwrap() {
        syn::punctuated::Pair::End(last_segment) => last_segment,
        _ => unreachable!(),
    };
    if &last_segment.ident.to_string() != "Vec" {
        return None;
    }
    let angle = match &last_segment.arguments {
        syn::PathArguments::AngleBracketed(angle) => {
            if angle.args.len() > 1 {
                unreachable!("Too many arguments for Vec!");
            }
            angle
        }
        _ => unreachable!("Vec with arguments that are not angle bracketed!"),
    };
    let last_arg = match angle.args.last().unwrap() {
        syn::punctuated::Pair::End(last_arg) => last_arg,
        _ => return None,
    };

    let arg_ty = match last_arg {
        syn::GenericArgument::Type(arg_ty) => arg_ty,
        _ => return None,
    };

    let arg_ty_path = match arg_ty {
        syn::Type::Path(arg_ty_path) => arg_ty_path,
        _ => return None,
    };

    if arg_ty_path.qself.is_some() {
        return None;
    }

    // Make sure that we don't deal with Vec<u8>:
    if arg_ty_path.path.is_ident("u8") {
        return None;
    }

    Some(arg_ty_path.path.clone())
}
