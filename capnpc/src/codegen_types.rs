// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use crate::codegen::{fmt, GeneratorContext};
use capnp::schema_capnp::{brand, node, type_};
use capnp::Error;
use std::collections::hash_map::HashMap;

#[derive(Copy, Clone, PartialEq)]
pub enum Leaf {
    Reader(&'static str),
    Builder(&'static str),
    Owned,
    Client,
    Server,
    ServerDispatch,
    Pipeline,
    GetType,
}

impl ::std::fmt::Display for Leaf {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        let display_string = match *self {
            Self::Reader(lt) => format!("Reader<{lt}>"),
            Self::Builder(lt) => format!("Builder<{lt}>"),
            Self::Owned => "Owned".to_string(),
            Self::Client => "Client".to_string(),
            Self::Server => "Server".to_string(),
            Self::ServerDispatch => "ServerDispatch".to_string(),
            Self::Pipeline => "Pipeline".to_string(),
            Self::GetType => "get_type".to_string(),
        };
        ::std::fmt::Display::fmt(&display_string, fmt)
    }
}

impl Leaf {
    fn bare_name(&self) -> &'static str {
        match *self {
            Self::Reader(_) => "Reader",
            Self::Builder(_) => "Builder",
            Self::Owned => "Owned",
            Self::Client => "Client",
            Self::Server => "Server",
            Self::ServerDispatch => "ServerDispatch",
            Self::GetType => "get_type",
            Self::Pipeline => "Pipeline",
        }
    }

    fn _have_lifetime(&self) -> bool {
        match self {
            &Self::Reader(_) | &Self::Builder(_) => true,
            &Self::Owned
            | &Self::Client
            | &Self::Server
            | &Self::ServerDispatch
            | &Self::GetType
            | &Self::Pipeline => false,
        }
    }
}

pub struct TypeParameterTexts {
    pub expanded_list: Vec<String>,
    pub params: String,
    pub where_clause: String,
    pub where_clause_with_static: String,
    pub pipeline_where_clause: String,
    pub phantom_data_value: String,
    pub phantom_data_type: String,
}

// this is a collection of helpers acting on a "Node" (most of them are Type definitions)
pub trait RustNodeInfo {
    fn parameters_texts(&self, ctx: &GeneratorContext) -> TypeParameterTexts;
}

// this is a collection of helpers acting on a "Type" (someplace where a Type is used, not defined)
pub trait RustTypeInfo {
    fn is_prim(&self) -> Result<bool, Error>;
    fn is_pointer(&self) -> Result<bool, Error>;
    fn is_parameter(&self) -> Result<bool, Error>;
    fn is_branded(&self) -> Result<bool, Error>;
    fn type_string(&self, ctx: &GeneratorContext, module: Leaf) -> Result<String, Error>;
}

impl<'a> RustNodeInfo for node::Reader<'a> {
    fn parameters_texts(&self, ctx: &GeneratorContext) -> TypeParameterTexts {
        if self.get_is_generic() {
            let params = get_type_parameters(ctx, self.get_id());
            let type_parameters = params
                .iter()
                .map(|param| param.to_string())
                .collect::<Vec<String>>()
                .join(",");
            let where_clause = "where ".to_string()
                + &*(params
                    .iter()
                    .map(|param| fmt!(ctx, "{param}: {capnp}::traits::Owned"))
                    .collect::<Vec<String>>()
                    .join(", ")
                    + " ");
            let where_clause_with_static = "where ".to_string()
                + &*(params
                    .iter()
                    .map(|param| fmt!(ctx, "{param}:'static + {capnp}::traits::Owned"))
                    .collect::<Vec<String>>()
                    .join(", ")
                    + " ");
            let pipeline_where_clause = "where ".to_string() + &*(params.iter().map(|param| {
                fmt!(ctx, "{param}: {capnp}::traits::Pipelined, <{param} as {capnp}::traits::Pipelined>::Pipeline: {capnp}::capability::FromTypelessPipeline")
            }).collect::<Vec<String>>().join(", ") + " ");
            let phantom_data_type = if params.len() == 1 {
                // omit parens to avoid linter error
                format!("_phantom: ::core::marker::PhantomData<{type_parameters}>")
            } else {
                format!("_phantom: ::core::marker::PhantomData<({type_parameters})>")
            };
            let phantom_data_value = "_phantom: ::core::marker::PhantomData,".to_string();

            TypeParameterTexts {
                expanded_list: params,
                params: type_parameters,
                where_clause,
                where_clause_with_static,
                pipeline_where_clause,
                phantom_data_type,
                phantom_data_value,
            }
        } else {
            TypeParameterTexts {
                expanded_list: vec![],
                params: "".to_string(),
                where_clause: "".to_string(),
                where_clause_with_static: "".to_string(),
                pipeline_where_clause: "".to_string(),
                phantom_data_type: "".to_string(),
                phantom_data_value: "".to_string(),
            }
        }
    }
}

impl<'a> RustTypeInfo for type_::Reader<'a> {
    fn type_string(&self, ctx: &GeneratorContext, module: Leaf) -> Result<String, Error> {
        let local_lifetime = match module {
            Leaf::Reader(lt) => lt,
            Leaf::Builder(lt) => lt,
            _ => "",
        };

        let lifetime_comma = if local_lifetime.is_empty() {
            "".to_string()
        } else {
            format!("{local_lifetime},")
        };

        match self.which()? {
            type_::Void(()) => Ok("()".to_string()),
            type_::Bool(()) => Ok("bool".to_string()),
            type_::Int8(()) => Ok("i8".to_string()),
            type_::Int16(()) => Ok("i16".to_string()),
            type_::Int32(()) => Ok("i32".to_string()),
            type_::Int64(()) => Ok("i64".to_string()),
            type_::Uint8(()) => Ok("u8".to_string()),
            type_::Uint16(()) => Ok("u16".to_string()),
            type_::Uint32(()) => Ok("u32".to_string()),
            type_::Uint64(()) => Ok("u64".to_string()),
            type_::Float32(()) => Ok("f32".to_string()),
            type_::Float64(()) => Ok("f64".to_string()),
            type_::Text(()) => Ok(fmt!(ctx, "{capnp}::text::{module}")),
            type_::Data(()) => Ok(fmt!(ctx, "{capnp}::data::{module}")),
            type_::Struct(st) => do_branding(
                ctx,
                st.get_type_id(),
                st.get_brand()?,
                module,
                &ctx.get_qualified_module(st.get_type_id()),
            ),
            type_::Interface(interface) => do_branding(
                ctx,
                interface.get_type_id(),
                interface.get_brand()?,
                module,
                &ctx.get_qualified_module(interface.get_type_id()),
            ),
            type_::List(ot1) => {
                let element_type = ot1.get_element_type()?;
                match element_type.which()? {
                    type_::Struct(_) => {
                        let inner = element_type.type_string(ctx, Leaf::Owned)?;
                        Ok(fmt!(
                            ctx,
                            "{capnp}::struct_list::{}<{lifetime_comma}{inner}>",
                            module.bare_name()
                        ))
                    }
                    type_::Enum(_) => {
                        let inner = element_type.type_string(ctx, Leaf::Owned)?;
                        Ok(fmt!(
                            ctx,
                            "{capnp}::enum_list::{}<{lifetime_comma}{inner}>",
                            module.bare_name()
                        ))
                    }
                    type_::List(_) => {
                        let inner = element_type.type_string(ctx, Leaf::Owned)?;
                        Ok(fmt!(
                            ctx,
                            "{capnp}::list_list::{}<{lifetime_comma}{inner}>",
                            module.bare_name()
                        ))
                    }
                    type_::Text(()) => Ok(format!("::capnp::text_list::{module}")),
                    type_::Data(()) => Ok(format!("::capnp::data_list::{module}")),
                    type_::Interface(_) => {
                        let inner = element_type.type_string(ctx, Leaf::Client)?;
                        Ok(fmt!(
                            ctx,
                            "{capnp}::capability_list::{}<{lifetime_comma}{inner}>",
                            module.bare_name()
                        ))
                    }
                    type_::AnyPointer(_) => {
                        Err(Error::failed("List(AnyPointer) is unsupported".to_string()))
                    }
                    _ => {
                        let inner = element_type.type_string(ctx, Leaf::Owned)?;
                        Ok(fmt!(
                            ctx,
                            "{capnp}::primitive_list::{}<{lifetime_comma}{inner}>",
                            module.bare_name()
                        ))
                    }
                }
            }
            type_::Enum(en) => Ok(ctx.get_qualified_module(en.get_type_id())),
            type_::AnyPointer(pointer) => match pointer.which()? {
                type_::any_pointer::Parameter(def) => {
                    let the_struct = &ctx.node_map[&def.get_scope_id()];
                    let parameters = the_struct.get_parameters()?;
                    let parameter = parameters.get(u32::from(def.get_parameter_index()));
                    let parameter_name = parameter.get_name()?.to_str()?;
                    match module {
                        Leaf::Owned => Ok(parameter_name.to_string()),
                        Leaf::Reader(lifetime) => Ok(fmt!(
                            ctx,
                            "<{parameter_name} as {capnp}::traits::Owned>::Reader<{lifetime}>"
                        )),
                        Leaf::Builder(lifetime) => Ok(fmt!(
                            ctx,
                            "<{parameter_name} as {capnp}::traits::Owned>::Builder<{lifetime}>"
                        )),
                        Leaf::Pipeline => Ok(fmt!(
                            ctx,
                            "<{parameter_name} as {capnp}::traits::Pipelined>::Pipeline"
                        )),
                        _ => Err(Error::unimplemented(
                            "unimplemented any_pointer leaf".to_string(),
                        )),
                    }
                }
                _ => match module {
                    Leaf::Reader(lifetime) => {
                        Ok(fmt!(ctx, "{capnp}::any_pointer::Reader<{lifetime}>"))
                    }
                    Leaf::Builder(lifetime) => {
                        Ok(fmt!(ctx, "{capnp}::any_pointer::Builder<{lifetime}>"))
                    }
                    _ => Ok(fmt!(ctx, "{capnp}::any_pointer::{module}")),
                },
            },
        }
    }

    fn is_parameter(&self) -> Result<bool, Error> {
        match self.which()? {
            type_::AnyPointer(pointer) => match pointer.which()? {
                type_::any_pointer::Parameter(_) => Ok(true),
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    fn is_branded(&self) -> Result<bool, Error> {
        match self.which()? {
            type_::Struct(st) => {
                let brand = st.get_brand()?;
                let scopes = brand.get_scopes()?;
                Ok(!scopes.is_empty())
            }
            _ => Ok(false),
        }
    }

    #[inline(always)]
    fn is_prim(&self) -> Result<bool, Error> {
        match self.which()? {
            type_::Int8(())
            | type_::Int16(())
            | type_::Int32(())
            | type_::Int64(())
            | type_::Uint8(())
            | type_::Uint16(())
            | type_::Uint32(())
            | type_::Uint64(())
            | type_::Float32(())
            | type_::Float64(())
            | type_::Void(())
            | type_::Bool(()) => Ok(true),
            _ => Ok(false),
        }
    }

    #[inline(always)]
    fn is_pointer(&self) -> Result<bool, Error> {
        Ok(matches!(
            self.which()?,
            type_::Text(())
                | type_::Data(())
                | type_::List(_)
                | type_::Struct(_)
                | type_::Interface(_)
                | type_::AnyPointer(_)
        ))
    }
}

pub fn do_branding(
    ctx: &GeneratorContext,
    node_id: u64,
    brand: brand::Reader,
    leaf: Leaf,
    the_mod: &str,
) -> Result<String, Error> {
    let scopes = brand.get_scopes()?;
    let mut brand_scopes = HashMap::new();
    for scope in scopes {
        brand_scopes.insert(scope.get_scope_id(), scope);
    }
    let brand_scopes = brand_scopes; // freeze
    let mut current_node_id = node_id;
    let mut accumulator: Vec<Vec<String>> = Vec::new();
    loop {
        let current_node = ctx.node_map[&current_node_id];
        let params = current_node.get_parameters()?;
        let mut arguments: Vec<String> = Vec::new();
        match brand_scopes.get(&current_node_id) {
            None => {
                for _ in params {
                    arguments.push(fmt!(ctx, "{capnp}::any_pointer::Owned"));
                }
            }
            Some(scope) => match scope.which()? {
                brand::scope::Inherit(()) => {
                    for param in params {
                        arguments.push(param.get_name()?.to_string()?);
                    }
                }
                brand::scope::Bind(bindings_list_opt) => {
                    let bindings_list = bindings_list_opt?;
                    assert_eq!(bindings_list.len(), params.len());
                    for binding in bindings_list {
                        match binding.which()? {
                            brand::binding::Unbound(()) => {
                                arguments.push(fmt!(ctx, "{capnp}::any_pointer::Owned"));
                            }
                            brand::binding::Type(t) => {
                                arguments.push(t?.type_string(ctx, Leaf::Owned)?);
                            }
                        }
                    }
                }
            },
        }
        accumulator.push(arguments);
        current_node_id = match ctx.node_parents.get(&current_node_id).copied() {
            Some(0) | None => break,
            Some(id) => id,
        };
    }

    // Now add a lifetime parameter if the leaf has one.
    match leaf {
        Leaf::Reader(lt) => accumulator.push(vec![lt.to_string()]),
        Leaf::Builder(lt) => accumulator.push(vec![lt.to_string()]),
        Leaf::ServerDispatch => accumulator.push(vec!["_T".to_string()]), // HACK
        _ => (),
    }

    accumulator.reverse();
    let accumulated = accumulator.concat();

    let arguments = if !accumulated.is_empty() {
        format!("<{}>", accumulated.join(","))
    } else {
        "".to_string()
    };

    let maybe_colons = if leaf == Leaf::ServerDispatch || leaf == Leaf::GetType {
        "::"
    } else {
        ""
    }; // HACK
    Ok(format!(
        "{the_mod}::{leaf}{maybe_colons}{arguments}",
        leaf = leaf.bare_name()
    ))
}

pub fn get_type_parameters(ctx: &GeneratorContext, node_id: u64) -> Vec<String> {
    let mut current_node_id = node_id;
    let mut accumulator: Vec<Vec<String>> = Vec::new();
    loop {
        let current_node = ctx.node_map[&current_node_id];
        let mut params = Vec::new();
        for param in current_node.get_parameters().unwrap() {
            params.push(param.get_name().unwrap().to_string().unwrap());
        }

        accumulator.push(params);
        current_node_id = match ctx.node_parents.get(&current_node_id).copied() {
            Some(0) | None => break,
            Some(id) => id,
        };
    }

    accumulator.reverse();
    accumulator.concat()
}
