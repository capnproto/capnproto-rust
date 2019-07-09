use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
// use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Index};
use syn::{FieldsNamed, Ident, Path};

use crate::util::{
    gen_list_read_iter, gen_list_write_iter, get_list, is_data, is_primitive, usize_to_u32_shim,
};

fn gen_type_write(field: &syn::Field, assign_defaults: impl Fn(&mut syn::Path)) -> TokenStream {
    match &field.ty {
        syn::Type::Path(type_path) => {
            if type_path.qself.is_some() {
                // Self qualifier?
                unimplemented!();
            }

            let mut path = type_path.path.clone();
            assign_defaults(&mut path);

            let name = &field.ident.as_ref().unwrap();

            if is_primitive(&path) {
                let set_method = syn::Ident::new(&format!("set_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    writer.reborrow().#set_method(self.#name);
                };
            }

            if path.is_ident("String") || is_data(&path) {
                let set_method = syn::Ident::new(&format!("set_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    writer.reborrow().#set_method(&self.#name);
                };
            }

            if let Some(inner_path) = get_list(&path) {
                let init_method = syn::Ident::new(&format!("init_{}", &name), name.span());
                let list_write_iter = gen_list_write_iter(&inner_path);

                // In the cases of more complicated types, list_builder needs to be mutable.
                let let_list_builder =
                    if is_primitive(&path) || path.is_ident("String") || is_data(&path) {
                        quote! { let list_builder }
                    } else {
                        quote! { let mut list_builder }
                    };

                let usize_to_u32 = usize_to_u32_shim();

                return quote_spanned! {field.span() =>
                    {
                        #usize_to_u32

                        #let_list_builder = {
                            writer
                            .reborrow()
                            .#init_method(usize_to_u32(self.#name.len()).unwrap())
                        };

                        for (index, item) in self.#name.iter().enumerate() {
                            #list_write_iter
                        }
                    }
                };
            }

            // Generic type:
            let init_method = syn::Ident::new(&format!("init_{}", &name), name.span());
            quote_spanned! {field.span() =>
                self.#name.write_capnp(&mut writer.reborrow().#init_method());
            }
        }
        _ => unimplemented!(),
    }
}

/// A shim allowing to merge cases where either
/// Result<T,Into<CapnoConvError>> or a T is returned.
fn capnp_result_shim() -> TokenStream {
    quote! {
        pub enum CapnpResult<T> {
            Ok(T),
            Err(CapnpConvError),
        }

        impl<T> CapnpResult<T> {
            pub fn into_result(self) -> Result<T, CapnpConvError> {
                match self {
                    CapnpResult::Ok(t) => Ok(t),
                    CapnpResult::Err(e) => Err(e),
                }
            }
        }

        impl<T> From<T> for CapnpResult<T> {
            fn from(input: T) -> Self {
                CapnpResult::Ok(input)
            }
        }

        impl<T, E> From<Result<T, E>> for CapnpResult<T>
        where
            E: Into<CapnpConvError>,
        {
            fn from(input: Result<T, E>) -> Self {
                match input {
                    Ok(t) => CapnpResult::Ok(t),
                    Err(e) => CapnpResult::Err(e.into()),
                }
            }
        }
    }
}

fn gen_type_read(field: &syn::Field, assign_defaults: impl Fn(&mut syn::Path)) -> TokenStream {
    match &field.ty {
        syn::Type::Path(type_path) => {
            if type_path.qself.is_some() {
                // Self qualifier?
                unimplemented!();
            }

            let mut path = type_path.path.clone();
            assign_defaults(&mut path);

            let name = &field.ident.as_ref().unwrap();

            if is_primitive(&path) {
                let get_method = syn::Ident::new(&format!("get_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    #name: reader.#get_method().into()
                };
            }

            if path.is_ident("String") || is_data(&path) {
                let get_method = syn::Ident::new(&format!("get_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    #name: reader.#get_method()?.into()
                };
            }

            if let Some(inner_path) = get_list(&path) {
                let get_method = syn::Ident::new(&format!("get_{}", &name), name.span());
                let list_read_iter = gen_list_read_iter(&inner_path);
                return quote_spanned! {field.span() =>
                    #name: {
                        let mut res_vec = Vec::new();
                        for item_reader in reader.#get_method()? {
                            // res_vec.push_back(read_named_relay_address(&named_relay_address)?);
                            #list_read_iter
                        }
                        res_vec
                    }
                };
            }

            // Generic type:
            let get_method = syn::Ident::new(&format!("get_{}", &name), name.span());
            let capnp_result = capnp_result_shim();
            quote_spanned! {field.span() =>
                #name: {
                    #capnp_result

                    let inner_reader = CapnpResult::from(reader.#get_method()).into_result()?;
                    <#path>::read_capnp(&inner_reader)?
                }
            }
        }
        _ => unimplemented!(),
    }
}

pub fn gen_write_capnp_named_struct(
    fields_named: &FieldsNamed,
    rust_struct: &Ident,
    capnp_struct: &Path,
    assign_defaults: impl Fn(&mut syn::Path),
) -> TokenStream {
    let recurse = fields_named
        .named
        .iter()
        .map(|field| gen_type_write(&field, &assign_defaults));

    quote! {
        impl<'a> WriteCapnp<'a> for #rust_struct {
            type WriterType = #capnp_struct::Builder<'a>;

            fn write_capnp(&self, writer: &mut Self::WriterType) {
                #(#recurse)*
            }
        }
    }
}

pub fn gen_read_capnp_named_struct(
    fields_named: &FieldsNamed,
    rust_struct: &Ident,
    capnp_struct: &Path,
    assign_defaults: impl Fn(&mut syn::Path),
) -> TokenStream {
    let recurse = fields_named
        .named
        .iter()
        .map(|field| gen_type_read(field, &assign_defaults));

    quote! {
        impl<'a> ReadCapnp<'a> for #rust_struct {
            type ReaderType = #capnp_struct::Reader<'a>;

            fn read_capnp(reader: &Self::ReaderType) -> Result<Self, CapnpConvError> {
                Ok(#rust_struct {
                    #(#recurse,)*
                })
            }
        }
    }
}
