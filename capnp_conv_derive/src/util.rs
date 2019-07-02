// use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Index};

use proc_macro2::TokenStream;
use quote::quote;

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
        || path.is_ident("f32")
        || path.is_ident("f64")
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

pub fn gen_list_write_iter(path: &syn::Path) -> TokenStream {
    if is_primitive(path) || path.is_ident("String") || is_data(path) {
        // A primitive list:
        quote! {
            list_builder
                .reborrow()
                .set(u32::try_from(index).unwrap(), item.clone());
        }
    } else {
        // Not a primitive list:
        quote! {
            let mut item_builder = list_builder
                .reborrow()
                .get(u32::try_from(index).unwrap());

            item.write_capnp(&mut item_builder);
        }
    }
    // TODO: It seems like we do not support List(List(...)) at the moment.
    // How to support it?
}

pub fn gen_list_read_iter(path: &syn::Path) -> TokenStream {
    if is_primitive(path) || path.is_ident("String") || is_data(path) {
        // A primitive list:
        quote! {
            res_vec.push(item_reader.into());
        }
    } else {
        // Not a primitive list:
        quote! {
            res_vec.push(#path::read_capnp(&item_reader)?);
        }
    }
    // TODO: It seems like we do not support List(List(...)) at the moment.
    // How to support it?
}
