use schema_capnp::*;

use codegen;

#[derive(Copy,Clone,PartialEq)]
pub enum Module { Reader, Builder, Owned, Client, Pipeline }
impl ::std::fmt::Display for Module {
    fn fmt(&self, fmt:&mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        ::std::fmt::Display::fmt(match self {
            &Module::Reader => "Reader",
            &Module::Builder => "Builder",
            &Module::Owned => "Owned",
            &Module::Client => "Client",
            &Module::Pipeline => "Pipeline",
        }, fmt)
    }
}

impl Module {
    fn have_lifetime(&self) -> bool {
        match self {
            &Module::Reader | &Module::Builder => true,
            &Module::Owned | &Module::Client | &Module::Pipeline => false,
        }
    }
}

pub struct TypeParameterTexts {
    pub expanded_list: Vec<String>,
    pub params: String,
    pub where_clause: String,
    pub where_clause_with_send: String,
    pub phantom_data: String
}

// this is a collection of helpers acting on a "Node" (most of them are Type definitions)
pub trait RustNodeInfo {
    fn type_string(&self, gen:&codegen::GeneratorContext, brand:&::schema_capnp::brand::Reader,
        scope:Option<&Vec<String>>, module:Module, lifetime:&str) -> String;

    // in rust, we have only nestable modules, and parameterizable types...
    // so a logically child struct will be a physical sibling to its logical parent. so it must
    // keep track of all parameterization on its own
    // this function recursively parse a node definition to find out the parameters used and inherited by scoping
    // and add them to the explicit parameters
    // the result is the actual parameters for the type to be generated
    fn expand_parameters(&self, gen:&::codegen::GeneratorContext) -> Vec<String>;

    fn parameters_texts(&self, gen:&::codegen::GeneratorContext) -> TypeParameterTexts;
}

// this is a collection of helpers acting on a "Type" (someplace where a Type is used, not defined)
pub trait RustTypeInfo {

    fn is_prim(&self) -> bool;
    fn is_parameterized(&self) -> bool;
    fn is_branded(&self) -> bool;
    fn type_string(&self, gen:&codegen::GeneratorContext,
        module:Module, lifetime:&str) -> String;
}

impl <'a> RustNodeInfo for node::Reader<'a> {

    fn expand_parameters(&self, gen:&::codegen::GeneratorContext) -> Vec<String> {
        let mut vec:Vec<String> = self.get_parameters().unwrap().iter().map(|p| p.get_name().unwrap().to_string()).collect();
        match self.which().unwrap() {
            ::schema_capnp::node::Struct(struct_reader) => {
                let fields = struct_reader.get_fields().unwrap();
                for field in fields.iter() {
                    match field.which().unwrap() {
                        ::schema_capnp::field::Slot(slot) => {
                            let typ = slot.get_type().unwrap().which().unwrap();
                            match typ {
                                ::schema_capnp::type_::Struct(st) => {
                                    let brand = st.get_brand().unwrap();
                                    let scopes = brand.get_scopes().unwrap();
                                    for scope in scopes.iter() {
                                        match scope.which().unwrap() {
                                            ::schema_capnp::brand::scope::Inherit(_) => {
                                                let parent_node = gen.node_map[&scope.get_scope_id()];
                                                for p in parent_node.get_parameters().unwrap().iter() {
                                                    let parameter_name = p.get_name().unwrap().to_string();
                                                    if !vec.contains(&parameter_name) {
                                                        vec.push(parameter_name);
                                                    }
                                                }
                                            },
                                            _ => {}
                                        }
                                    }
                                },
                                ::schema_capnp::type_::AnyPointer(any) => {
                                    match any.which().unwrap() {
                                        ::schema_capnp::type_::any_pointer::Parameter(def) => {
                                            let the_struct = &gen.node_map[&def.get_scope_id()];
                                            let parameters = the_struct.get_parameters().unwrap();
                                            let parameter = parameters.get(def.get_parameter_index() as u32);
                                            let parameter_name = parameter.get_name().unwrap().to_string();
                                            if !vec.contains(&parameter_name) {
                                                vec.push(parameter_name);
                                            }
                                        },
                                        _ => {}
                                    }
                                },
                                _ => {} // FIXME
                            }
                        },
                        _ => {} // FIXME
                    }
                }
            },
            ::schema_capnp::node::Interface(iface_reader) => {
                let methods = iface_reader.get_methods().unwrap();
                for method in methods.iter() {
                    for brand in vec!(method.get_param_brand().unwrap(), method.get_result_brand().unwrap()) {
                        for scope in brand.get_scopes().unwrap().iter() {
                            match scope.which().unwrap() {
                                ::schema_capnp::brand::scope::Inherit(_) => {
                                    let parent_node = gen.node_map[&scope.get_scope_id()];
                                    for p in parent_node.get_parameters().unwrap().iter() {
                                        let parameter_name = p.get_name().unwrap().to_string();
                                        if !vec.contains(&parameter_name) {
                                            vec.push(parameter_name);
                                        }
                                    }
                                },
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {} // FIXME
        }
        vec
    }

    fn parameters_texts(&self, gen:&::codegen::GeneratorContext) -> TypeParameterTexts {
        if self.get_is_generic() {
            let params = self.expand_parameters(&gen);
            let type_parameters = params.iter().map(|param| {
                format!("{}",param)
            }).collect::<Vec<String>>().connect(",");
            let where_clause = "where ".to_string() + &*(params.iter().map(|param| {
                format!("{}: for<'c> ::capnp::traits::Owned<'c>", param)
            }).collect::<Vec<String>>().connect(", ") + " ");
            let where_clause_with_send = "where ".to_string() + &*(params.iter().map(|param| {
                //format!("{}Reader:Send+FromPointerReader<'a>", param)
                format!("{}Reader:Send", param)
            }).collect::<Vec<String>>().connect(", ") + " ") + ", "
                + &*(params.iter().map(|param| {
                //format!("{}Builder:Send+FromPointerBuilder<'a>", param)
                format!("{}Builder:Send", param)
            }).collect::<Vec<String>>().connect(", ") + " ");
            let phantom_data = "_phantom: PhantomData,".to_string();

            TypeParameterTexts {
                expanded_list: params,
                params: type_parameters,
                where_clause: where_clause,
                where_clause_with_send: where_clause_with_send,
                phantom_data: phantom_data
            }
        } else {
            TypeParameterTexts {
                expanded_list: vec!(),
                params: "".to_string(),
                where_clause: "".to_string(),
                where_clause_with_send: "".to_string(),
                phantom_data: "".to_string(),
            }
        }
    }

    fn type_string(&self, gen:&codegen::GeneratorContext, brand:&::schema_capnp::brand::Reader,
            scope:Option<&Vec<String>>, module:Module, lifetime:&str) -> String {
        let the_mod = scope.unwrap_or_else( || &gen.scope_map[&self.get_id()]).connect("::");
        let mut reader_bindings:Vec<String> = vec!();
        let mut builder_bindings:Vec<String> = vec!();
        let parameters = self.expand_parameters(gen);
        for s in brand.get_scopes().unwrap().iter() {
            match s.which().unwrap() {
                brand::scope::Inherit(_) => {
                    let parent_node = gen.node_map[&s.get_scope_id()];
                    for p in parent_node.get_parameters().unwrap().iter() {
                        if parameters.contains(&p.get_name().unwrap().to_string()) {
                            reader_bindings.push(p.get_name().unwrap().to_string());
                            builder_bindings.push(p.get_name().unwrap().to_string());
                        }
                    }
                },
                brand::scope::Bind(b) => {
                    b.unwrap().iter().map(|binding|
                        match binding.which().unwrap() {
                            brand::binding::Type(Ok(t)) => {
                                reader_bindings.push(t.type_string(gen, Module::Owned, lifetime));
                                builder_bindings.push(t.type_string(gen, Module::Owned, lifetime));
                            }
                            _ => {}
                        }
                    ).count();
                },
            }
        };
        let local_lifetime = if module.have_lifetime() { lifetime } else { "" };
        let bracketed_lifetime = if local_lifetime == "" { "".to_string() } else {
            format!("<{}>", local_lifetime)
        };
        let lifetime_coma = if local_lifetime == "" { "".to_string() } else {
            format!("{},", local_lifetime)
        };
        let module_with_var = format!("{}{}", module, bracketed_lifetime);
        if parameters.len() == 0 {
            format!("{}::{}", the_mod, module_with_var)
        } else {
            if parameters.len() != reader_bindings.len() {
                reader_bindings.clear();
                builder_bindings.clear();
                for _ in 0 .. parameters.len() {
                    reader_bindings.push(format!("::capnp::any_pointer::Reader<{}>", lifetime));
                    builder_bindings.push(format!("::capnp::any_pointer::Builder<{}>", lifetime));
                }
            }
            format!("{}::{}<{}{}>", the_mod, module, lifetime_coma, reader_bindings.connect(","))
        }
    }
}

impl <'a> RustTypeInfo for type_::Reader<'a> {

    fn type_string(&self, gen:&codegen::GeneratorContext,
                   module:Module, lifetime:&str) -> String {
        use codegen_types::RustTypeInfo;

        let local_lifetime = if module.have_lifetime() { lifetime } else { "" };

        let bracketed_lifetime = if local_lifetime == "" { "".to_string() } else {
            format!("<{}>", local_lifetime)
        };
        let lifetime_coma = if local_lifetime == "" { "".to_string() } else {
            format!("{},", local_lifetime)
        };
        let module_with_var = format!("{}{}", module, bracketed_lifetime);

        match self.which().unwrap() {
            type_::Void(()) => "()".to_string(),
            type_::Bool(()) => "bool".to_string(),
            type_::Int8(()) => "i8".to_string(),
            type_::Int16(()) => "i16".to_string(),
            type_::Int32(()) => "i32".to_string(),
            type_::Int64(()) => "i64".to_string(),
            type_::Uint8(()) => "u8".to_string(),
            type_::Uint16(()) => "u16".to_string(),
            type_::Uint32(()) => "u32".to_string(),
            type_::Uint64(()) => "u64".to_string(),
            type_::Float32(()) => "f32".to_string(),
            type_::Float64(()) => "f64".to_string(),
            type_::Text(()) => format!("text::{}", module_with_var),
            type_::Data(()) => format!("data::{}", module_with_var),
            type_::Struct(st) => {
                gen.node_map[&st.get_type_id()].type_string(gen, &st.get_brand().unwrap(), None, module, local_lifetime)
            },
            type_::Interface(interface) => {
                gen.node_map[&interface.get_type_id()].type_string(gen, &interface.get_brand().unwrap(), None, Module::Client, local_lifetime)
            },
            type_::List(ot1) => {
                match ot1.get_element_type().unwrap().which() {
                    Err(_) => { panic!("unsupported type") },
                    Ok(type_::Struct(_)) => {
                        let inner = ot1.get_element_type().unwrap().type_string(gen, Module::Owned, "");
                        format!("struct_list::{}<{}{}>", module, lifetime_coma, inner)
                    },
                    Ok(type_::Enum(_)) => {
                        let inner = ot1.get_element_type().unwrap().type_string(gen, Module::Owned, "");
                        format!("enum_list::{}<{}{}>",module, lifetime_coma, inner)
                    },
                    Ok(type_::List(_)) => {
                        let inner = ot1.get_element_type().unwrap().type_string(gen, Module::Owned, "");
                        format!("list_list::{}<{}{}>", module, lifetime_coma, inner)
                    },
                    Ok(type_::Text(())) => {
                        format!("text_list::{}", module_with_var)
                    },
                    Ok(type_::Data(())) => {
                        format!("data_list::{}", module_with_var)
                    },
                    Ok(type_::Interface(_)) => {panic!("unimplemented") },
                    Ok(type_::AnyPointer(_)) => {panic!("List(AnyPointer) is unsupported")},
                    Ok(_) => {
                        let inner = ot1.get_element_type().unwrap().type_string(gen, Module::Owned, "");
                        format!("primitive_list::{}<{}{}>", module, lifetime_coma, inner)
                    },
                }
            },
            type_::Enum(en) => {
                let scope = &gen.scope_map[&en.get_type_id()];
                scope.connect("::").to_string()
            },
            type_::AnyPointer(pointer) => {
                match pointer.which().unwrap() {
                    type_::any_pointer::Parameter(def) => {
                        let the_struct = &gen.node_map[&def.get_scope_id()];
                        let parameters = the_struct.get_parameters().unwrap();
                        let parameter = parameters.get(def.get_parameter_index() as u32);
                        let parameter_name = parameter.get_name().unwrap();
                        match module {
                            Module::Owned => parameter_name.to_string(),
                            _ => {
                                format!("<{} as ::capnp::traits::Owned<{}>>::{}",
                                        parameter_name, lifetime, module)
                            }
                        }
                    },
                    _ => {
                        format!("::capnp::any_pointer::{}{}", module, bracketed_lifetime)
                    }
                }
            }
        }
    }

    fn is_parameterized(&self) -> bool {
        match self.which().unwrap() {
            type_::AnyPointer(pointer) => {
                match pointer.which().unwrap() {
                    type_::any_pointer::Parameter(_) => true,
                    _ => false
                }
            }
            _ => false
        }
    }

    fn is_branded(&self) -> bool {
        match self.which().unwrap() {
            type_::Struct(st) => {
                let brand = st.get_brand().unwrap();
                let scopes = brand.get_scopes().unwrap();
                scopes.len() > 0
            }
            _ => false
        }
    }

    #[inline(always)]
    fn is_prim(&self) -> bool {
        match self.which().unwrap() {
            type_::Int8(()) | type_::Int16(()) | type_::Int32(()) | type_::Int64(()) |
            type_::Uint8(()) | type_::Uint16(()) | type_::Uint32(()) | type_::Uint64(()) |
            type_::Float32(()) | type_::Float64(()) | type_::Void(()) | type_::Bool(()) => true,
            _ => false
        }
    }
}

