use schema_capnp::*;

use codegen;

#[derive(Copy,Clone,PartialEq)]
pub enum Module { Reader, Builder, Owned }
impl ::std::fmt::Display for Module {
    fn fmt(&self, fmt:&mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        ::std::fmt::Display::fmt(match self {
            &Module::Reader => "Reader",
            &Module::Builder => "Builder",
            &Module::Owned => "Owned",
        }, fmt)
    }
}

pub trait RustTypeInfo {

    fn is_prim(&self) -> bool;
    fn is_parameterized(&self) -> bool;
    fn is_branded(&self) -> bool;
    fn type_string(&self, gen:&codegen::GeneratorContext,
        module:Module, lifetime:&str) -> String;
}

impl <'a> RustTypeInfo for type_::Reader<'a> {

    fn type_string(&self, gen:&codegen::GeneratorContext,
                   module:Module, lifetime:&str) -> String {
        use codegen_types::RustTypeInfo;

        let bracketed_lifetime = if lifetime == "" { "".to_string() } else {
            format!("<{}>", lifetime)
        };
        let lifetime_coma = if lifetime == "" { "".to_string() } else {
            format!("{},", lifetime)
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
                let the_mod = gen.scope_map[&st.get_type_id()].connect("::");
                let mut reader_bindings = vec!();
                let mut builder_bindings = vec!();
                let brand = st.get_brand().unwrap();
                let scopes = brand.get_scopes().unwrap();
                for scope in scopes.iter() {
                    match scope.which().unwrap() {
                        brand::scope::Inherit(_) => {
                            let parent_node = gen.node_map[&scope.get_scope_id()];
                            reader_bindings.extend(parent_node.get_parameters().unwrap().iter().map(|p| p.get_name().unwrap().to_string()+"Reader"));
                            builder_bindings.extend(parent_node.get_parameters().unwrap().iter().map(|p| p.get_name().unwrap().to_string()+"Builder"));
                        },
                        brand::scope::Bind(b) => {
                            b.unwrap().iter().map(|binding|
                                match binding.which().unwrap() {
                                    brand::binding::Type(Ok(t)) => {
                                        reader_bindings.push(t.type_string(gen, Module::Reader, lifetime));
                                        builder_bindings.push(t.type_string(gen, Module::Builder, lifetime));
                                    }
                                    _ => {}
                                }
                            ).count();
                        },
                    }
                }

                if reader_bindings.len() == 0 {
                    format!("{}::{}", the_mod, module_with_var)
                } else {
                    format!("{}::{}<{}{},{}>", the_mod, module, lifetime_coma, reader_bindings.connect(","), builder_bindings.connect(","))
                }
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
            type_::Interface(interface) => {
                let the_mod = gen.scope_map[&interface.get_type_id()].connect("::");
                format!("{}::Client", the_mod)
            },
            type_::AnyPointer(pointer) => {
                match pointer.which().unwrap() {
                    type_::any_pointer::Parameter(def) => {
                        let the_struct = &gen.node_map[&def.get_scope_id()];
                        let parameters = the_struct.get_parameters().unwrap();
                        let parameter = parameters.get(def.get_parameter_index() as u32);
                        let parameter_name = parameter.get_name().unwrap();
                        format!("{}{}", parameter_name, module)
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
