use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
// use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident, Index};
use syn::{FieldsNamed, Ident, Path};

use crate::util::{gen_list_read_iter, gen_list_write_iter, get_list, is_data, is_primitive};

fn gen_type_write(field: &syn::Field) -> TokenStream {
    match &field.ty {
        syn::Type::Path(type_path) => {
            if type_path.qself.is_some() {
                // Self qualifier?
                unimplemented!();
            }

            let path = &type_path.path;

            let name = &field.ident.as_ref().unwrap();

            if is_primitive(path) {
                let set_method = syn::Ident::new(&format!("set_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    writer.reborrow().#set_method(self.#name);
                };
            }

            if path.is_ident("String") || is_data(path) {
                let set_method = syn::Ident::new(&format!("set_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    writer.reborrow().#set_method(&self.#name);
                };
            }

            if let Some(inner_path) = get_list(path) {
                let init_method = syn::Ident::new(&format!("init_{}", &name), name.span());
                let list_write_iter = gen_list_write_iter(&inner_path);

                // In the cases of more complicated types, list_builder needs to be mutable.
                let let_list_builder =
                    if is_primitive(path) || path.is_ident("String") || is_data(path) {
                        quote! { let list_builder }
                    } else {
                        quote! { let mut list_builder }
                    };

                return quote_spanned! {field.span() =>
                    #let_list_builder = writer
                        .reborrow()
                        .#init_method(u32::try_from(self.#name.len()).unwrap());

                    for (index, item) in self.#name.iter().enumerate() {
                        #list_write_iter
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

fn gen_type_read(field: &syn::Field) -> TokenStream {
    match &field.ty {
        syn::Type::Path(type_path) => {
            if type_path.qself.is_some() {
                // Self qualifier?
                unimplemented!();
            }

            let path = &type_path.path;

            let name = &field.ident.as_ref().unwrap();

            if is_primitive(path) {
                let get_method = syn::Ident::new(&format!("get_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    #name: reader.#get_method().into()
                };
            }

            if path.is_ident("String") || is_data(path) {
                let get_method = syn::Ident::new(&format!("get_{}", &name), name.span());
                return quote_spanned! {field.span() =>
                    #name: reader.#get_method()?.into()
                };
            }

            if let Some(inner_path) = get_list(path) {
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
            quote_spanned! {field.span() =>
                #name: {
                    let inner_reader = CapnpResult::from(reader.#get_method()).into_result()?;
                    #type_path::read_capnp(&inner_reader)?
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
) -> TokenStream {
    let recurse = fields_named
        .named
        .iter()
        .map(|field| gen_type_write(&field));

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
) -> TokenStream {
    let recurse = fields_named.named.iter().map(|field| gen_type_read(field));

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
