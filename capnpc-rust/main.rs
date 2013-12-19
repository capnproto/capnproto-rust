/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[feature(globs)];

#[crate_id="capnpc-rust"];
#[link(name = "capnpc-rust", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnp;

use capnp::*;

pub mod schema_capnp;

fn element_size_str (elementSize : schema_capnp::ElementSize::Reader) -> ~ str {
    use schema_capnp::ElementSize::*;
    match elementSize {
        Empty => ~"VOID",
        Bit => ~"BIT",
        Byte => ~"BYTE",
        TwoBytes => ~"TWO_BYTES",
        FourBytes => ~"FOUR_BYTES",
        EightBytes => ~"EIGHT_BYTES",
        Pointer => ~"POINTER",
        InlineComposite => ~"INLINE_COMPOSITE"
    }
}

fn element_size (typ : schema_capnp::Type::Which) -> schema_capnp::ElementSize::Reader {
    use schema_capnp::Type::*;
    use schema_capnp::ElementSize::*;
    match typ {
        Void(()) => Empty,
        Bool(()) => Bit,
        Int8(()) => Byte,
        Int16(()) => TwoBytes,
        Int32(()) => FourBytes,
        Int64(()) => EightBytes,
        Uint8(()) => Byte,
        Uint16(()) => TwoBytes,
        Uint32(()) => FourBytes,
        Uint64(()) => EightBytes,
        Float32(()) => FourBytes,
        Float64(()) => EightBytes,
        _ => fail!("not primitive")
    }
}

fn prim_type_str (typ : schema_capnp::Type::Which) -> ~str {
    use schema_capnp::Type::*;
    match typ {
        Void(()) => ~"()",
        Bool(()) => ~"bool",
        Int8(()) => ~"i8",
        Int16(()) => ~"i16",
        Int32(()) => ~"i32",
        Int64(()) => ~"i64",
        Uint8(()) => ~"u8",
        Uint16(()) => ~"u16",
        Uint32(()) => ~"u32",
        Uint64(()) => ~"u64",
        Float32(()) => ~"f32",
        Float64(()) => ~"f64",
        Enum(_) => ~"u16",
        _ => fail!("not primitive")
    }
}

#[allow(dead_code)]
fn camel_to_upper_case(s : &str) -> ~str {
    use std::ascii::*;
    let mut result_chars : ~[char] = ~[];
    for c in s.chars() {
        assert!(std::char::is_alphanumeric(c), format!("not alphanumeric '{}'", c));
        if (std::char::is_uppercase(c)) {
            result_chars.push('_');
        }
        result_chars.push((c as u8).to_ascii().to_upper().to_char());
    }
    return std::str::from_chars(result_chars);
}

fn camel_to_snake_case(s : &str) -> ~str {
    use std::ascii::*;
    let mut result_chars : ~[char] = ~[];
    for c in s.chars() {
        assert!(std::char::is_alphanumeric(c), format!("not alphanumeric '{}', i.e. {}", c, c as uint));
        if (std::char::is_uppercase(c)) {
            result_chars.push('_');
        }
        result_chars.push((c as u8).to_ascii().to_lower().to_char());
    }
    return std::str::from_chars(result_chars);
}

fn capitalize_first_letter(s : &str) -> ~str {
    use std::ascii::*;
    let mut result_chars : ~[char] = ~[];
    for c in s.chars() { result_chars.push(c) }
    result_chars[0] = (result_chars[0] as u8).to_ascii().to_upper().to_char();
    return std::str::from_chars(result_chars);
}

#[test]
fn test_camel_to_upper_case() {
    assert_eq!(camel_to_upper_case("fooBar"), ~"FOO_BAR");
    assert_eq!(camel_to_upper_case("fooBarBaz"), ~"FOO_BAR_BAZ");
    assert_eq!(camel_to_upper_case("helloWorld"), ~"HELLO_WORLD");
}

#[test]
fn test_camel_to_snake_case() {
    assert_eq!(camel_to_snake_case("fooBar"), ~"foo_bar");
    assert_eq!(camel_to_snake_case("fooBarBaz"), ~"foo_bar_baz");
    assert_eq!(camel_to_snake_case("helloWorld"), ~"hello_world");
    assert_eq!(camel_to_snake_case("uint32Id"), ~"uint32_id");
}

#[deriving(Eq)]
enum FormattedText {
    Indent(~FormattedText),
    Branch(~[FormattedText]),
    Line(~str),
    BlankLine
}

fn to_lines(ft : &FormattedText, indent : uint) -> ~[~str] {
    match *ft {
        Indent (ref ft) => {
            return to_lines(*ft, indent + 1);
        }
        Branch (ref fts) => {
            return fts.flat_map(|ft| {to_lines(ft, indent)});
        }
        Line(ref s) => {
            let mut s1 : ~str = std::str::from_chars(
                std::vec::from_elem(indent * 2, ' '));
            s1.push_str(*s);
            return ~[s1];
        }
        BlankLine => return ~[~""]
    }
}

fn stringify(ft : & FormattedText) -> ~str {
    let mut result = to_lines(ft, 0).connect("\n");
    result.push_str("\n");
    return result;
}

fn append_name (names : &[~str], name : ~str) -> ~[~str] {
    let mut result : ~[~str] = ~[];
    for n in names.iter() {
        result.push(n.to_owned());
    }
    result.push(name);
    return result;
}

//type NodeMap = std::hashmap::HashMap<u64, schema_capnp::Node::Reader>;

fn populate_scope_map(nodeMap : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                      scopeMap : &mut std::hashmap::HashMap<u64, ~[~str]>,
                      rootName : &str,
                      nodeId : u64) {
    let nodeReader = nodeMap.get(&nodeId);

    let nestedNodes = nodeReader.get_nested_nodes();
    for ii in range(0, nestedNodes.size()) {
        let nestedNode = nestedNodes[ii];
        let id = nestedNode.get_id();
        let name = nestedNode.get_name().to_owned();

        let scopeNames = match scopeMap.find(&nodeId) {
            Some(names) => append_name(*names, name),
            None => ~[rootName.to_owned(), name]
        };
        scopeMap.insert(id, scopeNames);
        populate_scope_map(nodeMap, scopeMap, rootName, id);
    }

    match nodeReader.which() {
        Some(schema_capnp::Node::Struct(structReader)) => {
            let fields = structReader.get_fields();
            for jj in range(0, fields.size()) {
                let field = fields[jj];
                match field.which() {
                    Some(schema_capnp::Field::Group(group)) => {
                        let id = group.get_type_id();
                        let name = capitalize_first_letter(field.get_name());
                        let scopeNames = match scopeMap.find(&nodeId) {
                            Some(names) => append_name(*names, name),
                            None => ~[rootName.to_owned(), name]
                        };

                        scopeMap.insert(id, scopeNames);
                        populate_scope_map(nodeMap, scopeMap, rootName, id);
                    }
                    _ => {}
                }
            }
        }
        _ => {  }
    }
}

fn generate_import_statements(rootName : &str) -> FormattedText {
    Branch(~[
        Line(~"use std;"),
        Line(~"use capnp::blob::{Text, Data};"),
        Line(~"use capnp::layout;"),
        Line(~"use capnp::any::AnyPointer;"),
        Line(~"use capnp::list::{PrimitiveList, ToU16, EnumList, StructList, TextList, DataList, ListList};"),
        Line(format!("use {};", rootName))
    ])
}

fn list_list_type_param(scope_map : &std::hashmap::HashMap<u64, ~[~str]>,
                        typ : schema_capnp::Type::Reader, is_reader: bool) -> ~str {
    use schema_capnp::Type;
    let module = if is_reader { "Reader" } else { "Builder" };
    match typ.which() {
        None => fail!("unsupported type"),
        Some(t) => {
            match t {
                Type::Void(()) | Type::Bool(()) | Type::Int8(()) |
                    Type::Int16(()) | Type::Int32(()) | Type::Int64(()) |
                    Type::Uint8(()) | Type::Uint16(()) | Type::Uint32(()) |
                    Type::Uint64(()) | Type::Float32(()) | Type::Float64(()) => {
                    format!("PrimitiveList::{}<'a, {}>", module, prim_type_str(t))
                }
                Type::Enum(en) => {
                    let theMod = scope_map.get(&en.get_type_id()).connect("::");
                    format!("EnumList::{}<'a,{}::Reader>", module, theMod)
                }
                Type::Text(()) => {
                    format!("TextList::{}<'a>", module)
                }
                Type::Data(()) => {
                    format!("DataList::{}<'a>", module)
                }
                Type::Struct(st) => {
                    format!("StructList::{}<'a, {}::{}>", module,
                            scope_map.get(&st.get_type_id()).connect("::"), module)
                }
                Type::List(t) => {
                    let inner = list_list_type_param(scope_map, t.get_element_type(), is_reader);
                    format!("ListList::{}<'a, {}>", module, inner)
                }
                Type::AnyPointer(()) => {
                    fail!("List(AnyPointer) is unsupported");
                }
                Type::Interface(_i) => {
                    fail!("unimplemented");
                }
            }
        }
    }
}

fn getter_text (_nodeMap : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
               scopeMap : &std::hashmap::HashMap<u64, ~[~str]>,
               field : &schema_capnp::Field::Reader,
               isReader : bool)
    -> (~str, FormattedText) {

    use schema_capnp::*;

    match field.which() {
        None => fail!("unrecognized field type"),
        Some(Field::Group(group)) => {
            let scope = scopeMap.get(&group.get_type_id());
            let theMod = scope.connect("::");
            if (isReader) {
                return (format!("{}::Reader<'a>", theMod),
                        Line(format!("{}::Reader::new(self.reader)", theMod)));
            } else {
                return (format!("{}::Builder<'a>", theMod),
                        Line(format!("{}::Builder::new(self.builder)", theMod)));
            }
        }
        Some(Field::Slot(regField)) => {

            let typ = regField.get_type();
            let offset = regField.get_offset() as uint;
            //    let defaultValue = field.getDefaultValue();

            let member = if (isReader) { "reader" } else { "builder" };
            let module = if (isReader) { "Reader" } else { "Builder" };
            let moduleWithVar = if (isReader) { "Reader<'a>" } else { "Builder<'a>" };

            match typ.which() {
                Some(Type::Void(())) => { return (~"()", Line(~"()"))}
                Some(Type::Bool(())) => {
                    return (~"bool", Line(format!("self.{}.get_bool_field({})",
                                                  member, offset)))
                }
                Some(Type::Int8(())) => return common_case("i8", member, offset),
                Some(Type::Int16(())) => return common_case("i16", member, offset),
                Some(Type::Int32(())) => return common_case("i32", member, offset),
                Some(Type::Int64(())) => return common_case("i64", member, offset),
                Some(Type::Uint8(())) => return common_case("u8", member, offset),
                Some(Type::Uint16(())) => return common_case("u16", member, offset),
                Some(Type::Uint32(())) => return common_case("u32", member, offset),
                Some(Type::Uint64(())) => return common_case("u64", member, offset),
                Some(Type::Float32(())) => return common_case("f32", member, offset),
                Some(Type::Float64(())) => return common_case("f64", member, offset),
                Some(Type::Text(())) => {
                    return (format!("Text::{}", moduleWithVar),
                            Line(format!("self.{}.get_pointer_field({}).get_text(std::ptr::null(), 0)",
                                      member, offset)));
                }
                Some(Type::Data(())) => {
                    return (format!("Data::{}", moduleWithVar),
                            Line(format!("self.{}.get_pointer_field({}).get_data(std::ptr::null(), 0)",
                                      member, offset)));
                }
                Some(Type::List(ot1)) => {
                    match ot1.get_element_type().which() {
                        None => { fail!("unsupported type") }
                        Some(Type::Struct(st)) => {
                            let scope = scopeMap.get(&st.get_type_id());
                            let theMod = scope.connect("::");
                            return (format!("StructList::{}<'a,{}::{}<'a>>", module, theMod, module),
                                    Line(format!("StructList::{}::new(self.{}.get_pointer_field({}).get_list({}::STRUCT_SIZE.preferred_list_encoding, std::ptr::null()))",
                                                 module, member, offset, theMod))
                                    );
                        }
                        Some(Type::Enum(e)) => {
                            let scope = scopeMap.get(&e.get_type_id());
                            let theMod = scope.connect("::");
                            let fullModuleName = format!("{}::Reader", theMod);
                            return (format!("EnumList::{}<'a,{}>",module,fullModuleName),
                                    Line(format!("EnumList::{}::new(self.{}.get_pointer_field({}).get_list(layout::TWO_BYTES, std::ptr::null()))",
                                         module, member, offset)));
                        }
                        Some(Type::List(t1)) => {
                            let type_param = list_list_type_param(scopeMap, t1.get_element_type(), isReader);
                            return (format!("ListList::{}<'a,{}>", module, type_param),
                                    Line(format!("ListList::{}::new(self.{}.get_pointer_field({}).get_list(layout::POINTER, std::ptr::null()))",
                                                 module, member, offset)))
                        }
                        Some(Type::Text(())) => {
                            return (format!("TextList::{}<'a>", module),
                                    Line(format!("TextList::{}::new(self.{}.get_pointer_field({}).get_list(layout::POINTER, std::ptr::null()))",
                                                 module, member, offset)))
                        }
                        Some(Type::Data(())) => {
                            return (format!("DataList::{}<'a>", module),
                                    Line(format!("DataList::{}::new(self.{}.get_pointer_field({}).get_list(layout::POINTER, std::ptr::null()))",
                                                 module, member, offset)))
                        }
                        Some(Type::Interface(_)) => {fail!("unimplemented") }
                        Some(Type::AnyPointer(())) => {fail!("List(AnyPointer) is unsupported")}
                        Some(primType) => {
                            let typeStr = prim_type_str(primType);
                            let sizeStr = element_size_str(element_size(primType));
                            return
                                (format!("PrimitiveList::{}<'a,{}>", module, typeStr),
                                 Line(format!("PrimitiveList::{}::new(self.{}.get_pointer_field({}).get_list(layout::{}, std::ptr::null()))",
                                           module, member, offset, sizeStr)))
                        }
                    }
                }
                Some(Type::Enum(en)) => {
                    let id = en.get_type_id();
                    let scope = scopeMap.get(&id);
                    let theMod = scope.connect("::");
                    return
                        (format!("Option<{}::Reader>", theMod), // Enums don't have builders.
                         Branch(
                            ~[Line(format!("FromPrimitive::from_u16(self.{}.get_data_field::<u16>({}))",
                                        member, offset))
                              ]));
                }
                Some(Type::Struct(st)) => {
                    let theMod = scopeMap.get(&st.get_type_id()).connect("::");
                    let middleArg = if (isReader) {~""} else {format!("{}::STRUCT_SIZE,", theMod)};
                    return (format!("{}::{}", theMod, moduleWithVar),
                            Line(format!("{}::{}::new(self.{}.get_pointer_field({}).get_struct({} std::ptr::null()))",
                                      theMod, module, member, offset, middleArg)))
                }
                Some(Type::Interface(_)) => {
                        return (~"TODO", Line(~"TODO"));
                }
                Some(Type::AnyPointer(())) => {
                    return (format!("AnyPointer::{}<'a>", module),
                            Line(format!("AnyPointer::{}::new(self.{}.get_pointer_field({}))",
                                         module, member, offset)))
                }
                None => {
                    // XXX should probably silently ignore, instead.
                    fail!("unrecognized type")
                }
            }
        }
    }

    fn common_case(typ: &str, member: &str, offset: uint) -> (~str, FormattedText) {
        return (typ.to_owned(),
                Line(format!("self.{}.get_data_field::<{}>({})",
                             member, typ, offset)))
    }
}

fn zero_fields_of_group(node_map : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                        node_id : u64
                        ) -> FormattedText {
    use schema_capnp::*;
    match node_map.get(&node_id).which() {
        Some(Node::Struct(st)) => {
            let mut result = ~[];
            if (st.get_discriminant_count() != 0) {
                result.push(
                    Line(format!("self.builder.set_data_field::<u16>({}, 0);",
                                 st.get_discriminant_offset())));
            }
            let fields = st.get_fields();
            for ii in range(0, fields.size()) {
                match fields[ii].which() {
                    None => {fail!()}
                    Some(Field::Group(group)) => {
                        result.push(zero_fields_of_group(node_map, group.get_type_id()));
                    }
                    Some(Field::Slot(slot)) => {
                        match slot.get_type().which(){
                            Some(typ) => {
                                match typ {
                                    Type::Void(()) => {}
                                    Type::Bool(()) => {
                                        let line = Line(format!("self.builder.set_bool_field({}, false);",
                                                         slot.get_offset()));
                                        // PERF could dedup more efficiently
                                        if !result.contains(&line) { result.push(line) }
                                    }
                                    Type::Int8(()) |
                                        Type::Int16(()) | Type::Int32(()) | Type::Int64(()) |
                                        Type::Uint8(()) | Type::Uint16(()) | Type::Uint32(()) |
                                        Type::Uint64(()) | Type::Float32(()) | Type::Float64(())
                                        | Type::Enum(_) => {
                                        let line = Line(format!("self.builder.set_data_field::<{}>({}, 0);",
                                                         prim_type_str(typ),
                                                         slot.get_offset()));
                                        // PERF could dedup more efficiently
                                        if !result.contains(&line) { result.push(line) }
                                    }
                                    Type::Struct(_) | Type::List(_) | Type::Text(()) | Type::Data(()) |
                                        Type::AnyPointer(()) => {
                                        let line = Line(format!("self.builder.get_pointer_field({}).clear();",
                                                                slot.get_offset()));
                                        // PERF could dedup more efficiently
                                        if !result.contains(&line) { result.push(line) }
                                    }
                                    Type::Interface(_) => { fail!() }
                                }
                            }
                            None => {fail!()}
                        }
                    }
                }
            }
            return Branch(result);
        }
        _ => { fail!("expected a struct") }
    }
}

fn generate_setter(node_map : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                  scopeMap : &std::hashmap::HashMap<u64, ~[~str]>,
                  discriminantOffset : u32,
                  styled_name : &str,
                  field :&schema_capnp::Field::Reader) -> FormattedText {

    use schema_capnp::*;

    let mut setter_interior = ~[];
    let mut setter_param = ~"value";
    let mut initter_interior = ~[];
    let mut initter_params = ~[];

    let discriminantValue = field.get_discriminant_value();
    if (discriminantValue != Field::NO_DISCRIMINANT) {
        setter_interior.push(
            Line(format!("self.builder.set_data_field::<u16>({}, {});",
                         discriminantOffset as uint,
                         discriminantValue as uint)));
        initter_interior.push(
            Line(format!("self.builder.set_data_field::<u16>({}, {});",
                         discriminantOffset as uint,
                         discriminantValue as uint)));
    }

    let (maybe_reader_type, maybe_builder_type) : (Option<~str>, Option<~str>) = match field.which() {
        None => fail!("unrecognized field type"),
        Some(Field::Group(group)) => {
            let scope = scopeMap.get(&group.get_type_id());
            let theMod = scope.connect("::");

            initter_interior.push(zero_fields_of_group(node_map, group.get_type_id()));

            initter_interior.push(Line(format!("{}::Builder::new(self.builder)", theMod)));

            //setter_interior.push(Line(format!("Builder { builder : }")));

            (None, Some(format!("{}::Builder<'a>", theMod)))
        }
        Some(Field::Slot(regField)) => {
            let offset = regField.get_offset() as uint;

            let common_case = |typ: &str| {
                setter_interior.push(Line(format!("self.builder.set_data_field::<{}>({}, value);",
                                                  typ, offset)));
                (Some(typ.to_owned()), None)
            };

            match regField.get_type().which() {
                Some(Type::Void(())) => {
                    setter_param = ~"_value";
                    (Some(~"()"), None)
                }
                Some(Type::Bool(())) => {
                    setter_interior.push(Line(format!("self.builder.set_bool_field({}, value);", offset)));
                    (Some(~"bool"), None)
                }
                Some(Type::Int8(())) => common_case("i8"),
                Some(Type::Int16(())) => common_case("i16"),
                Some(Type::Int32(())) => common_case("i32"),
                Some(Type::Int64(())) => common_case("i64"),
                Some(Type::Uint8(())) => common_case("u8"),
                Some(Type::Uint16(())) => common_case("u16"),
                Some(Type::Uint32(())) => common_case("u32"),
                Some(Type::Uint64(())) => common_case("u64"),
                Some(Type::Float32(())) => common_case("f32"),
                Some(Type::Float64(())) => common_case("f64"),
                Some(Type::Text(())) => {
                    setter_interior.push(Line(format!("self.builder.get_pointer_field({}).set_text(value);",
                                                      offset)));
                    initter_interior.push(Line(format!("self.builder.get_pointer_field({}).init_text(size)",
                                                       offset)));
                    initter_params.push("size : uint");
                    (Some(~"Text::Reader<'a>"), Some(~"Text::Builder<'a>"))
                }
                Some(Type::Data(())) => {
                    setter_interior.push(Line(format!("self.builder.get_pointer_field({}).set_data(value);",
                                                      offset)));
                    initter_interior.push(Line(format!("self.builder.get_pointer_field({}).init_data(size)",
                                                       offset)));
                    initter_params.push("size : uint");
                    (Some(~"Data::Reader<'a>"), Some(~"Data::Builder<'a>"))
                }
                Some(Type::List(ot1)) => {
                    setter_interior.push(
                        Line(format!("self.builder.get_pointer_field({}).set_list(&value.reader)",
                                     offset)));

                    initter_params.push("size : uint");
                    match ot1.get_element_type().which() {
                        None => fail!("unsupported type"),
                        Some(t1) => {
                            match t1 {
                                Type::Void(()) | Type::Bool(()) | Type::Int8(()) |
                                    Type::Int16(()) | Type::Int32(()) | Type::Int64(()) |
                                    Type::Uint8(()) | Type::Uint16(()) | Type::Uint32(()) |
                                    Type::Uint64(()) | Type::Float32(()) | Type::Float64(()) => {

                                    let typeStr = prim_type_str(t1);
                                    let sizeStr = element_size_str(element_size(t1));

                                    initter_interior.push(Line(format!("PrimitiveList::Builder::<'a,{}>::new(",
                                                               typeStr)));
                                    initter_interior.push(
                                        Indent(~Line(format!("self.builder.get_pointer_field({}).init_list(layout::{},size)",
                                                          offset, sizeStr))));
                                    initter_interior.push(Line(~")"));

                                    (Some(format!("PrimitiveList::Reader<'a,{}>", typeStr)),
                                     Some(format!("PrimitiveList::Builder<'a,{}>", typeStr)))
                                }
                                Type::Enum(e) => {
                                    let id = e.get_type_id();
                                    let scope = scopeMap.get(&id);
                                    let theMod = scope.connect("::");
                                    let typeStr = format!("{}::Reader", theMod);
                                    initter_interior.push(Line(format!("EnumList::Builder::<'a, {}>::new(",
                                                            typeStr)));
                                    initter_interior.push(
                                        Indent(
                                            ~Line(
                                                format!("self.builder.get_pointer_field({}).init_list(layout::TWO_BYTES,size)",
                                                     offset))));
                                    initter_interior.push(Line(~")"));
                                    (Some(format!("EnumList::Reader<'a,{}>", typeStr)),
                                     Some(format!("EnumList::Builder<'a,{}>", typeStr)))
                                }
                                Type::Struct(st) => {
                                    let id = st.get_type_id();
                                    let scope = scopeMap.get(&id);
                                    let theMod = scope.connect("::");

                                    initter_interior.push(Line(format!("StructList::Builder::<'a, {}::Builder<'a>>::new(", theMod)));
                                    initter_interior.push(
                                       Indent(
                                          ~Line(
                                             format!("self.builder.get_pointer_field({}).init_struct_list(size, {}::STRUCT_SIZE))",
                                                  offset, theMod))));

                                    (Some(format!("StructList::Reader<'a,{}::Reader<'a>>", theMod)),
                                     Some(format!("StructList::Builder<'a,{}::Builder<'a>>", theMod)))
                                }
                                Type::Text(()) => {
                                    initter_interior.push(
                                        Line(format!("TextList::Builder::<'a>::new(self.builder.get_pointer_field({}).init_list(layout::POINTER, size))", offset)));

                                    (Some(format!("TextList::Reader<'a>")),
                                     Some(format!("TextList::Builder<'a>")))
                                }
                                Type::Data(()) => {
                                    initter_interior.push(
                                        Line(format!("DataList::Builder::<'a>::new(self.builder.get_pointer_field({}).init_list(layout::POINTER, size))", offset)));

                                    (Some(format!("DataList::Reader<'a>")),
                                     Some(format!("DataList::Builder<'a>")))
                                }
                                Type::List(t1) => {
                                    let type_param = list_list_type_param(scopeMap, t1.get_element_type(), false);
                                    initter_interior.push(
                                        Line(format!("ListList::Builder::<'a,{}>::new(self.builder.get_pointer_field({}).init_list(layout::POINTER,size))",
                                                     type_param, offset)));

                                    (Some(format!("ListList::Reader<'a, {}>",
                                             list_list_type_param(scopeMap, t1.get_element_type(), true))),
                                     Some(format!("ListList::Builder<'a, {}>", type_param)))
                                }
                                Type::AnyPointer(()) => {fail!("List(AnyPointer) not supported")}
                                Type::Interface(_) => { fail!("unimplemented") }
                            }
                        }
                    }
                }
                Some(Type::Enum(e)) => {
                    let id = e.get_type_id();
                    let theMod = scopeMap.get(&id).connect("::");
                    setter_interior.push(
                        Line(format!("self.builder.set_data_field::<u16>({}, value as u16)",
                                     offset)));
                    (Some(format!("{}::Reader", theMod)), None)
                }
                Some(Type::Struct(st)) => {
                    let theMod = scopeMap.get(&st.get_type_id()).connect("::");
                    setter_interior.push(
                        Line(format!("self.builder.get_pointer_field({}).set_struct(&value.reader)", offset)));
                    initter_interior.push(
                      Line(format!("{}::Builder::new(self.builder.get_pointer_field({}).init_struct({}::STRUCT_SIZE))",
                                theMod, offset, theMod)));
                    (Some(format!("{}::Reader<'a>", theMod)), Some(format!("{}::Builder<'a>", theMod)))
                }
                Some(Type::Interface(_)) => {
                    fail!("unimplemented");
                }
                Some(Type::AnyPointer(())) => {
                    initter_interior.push(Line(format!("let result = AnyPointer::Builder::new(self.builder.get_pointer_field({}));",
                                               offset)));
                    initter_interior.push(Line(~"result.clear();"));
                    initter_interior.push(Line(~"result"));
                    (None, Some(~"AnyPointer::Builder<'a>"))
                }
                None => { fail!("unrecognized type") }
            }
        }
    };
    let mut result = ~[];
    match maybe_reader_type {
        Some(reader_type) => {
            result.push(Line(~"#[inline]"));
            result.push(Line(format!("pub fn set_{}(&self, {} : {}) \\{",
                                     styled_name, setter_param, reader_type)));
            result.push(Indent(~Branch(setter_interior)));
            result.push(Line(~"}"));
        }
        None => {}
    }
    match maybe_builder_type {
        Some(builder_type) => {
            result.push(Line(~"#[inline]"));
            let args = initter_params.connect(", ");
            result.push(Line(format!("pub fn init_{}(&self, {}) -> {} \\{",
                                     styled_name, args, builder_type)));
            result.push(Indent(~Branch(initter_interior)));
            result.push(Line(~"}"));
        }
        None => {}
    }
    return Branch(result);
}


// return (the 'Which' enum, the 'which()' accessor)
fn generate_union(nodeMap : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                  scopeMap : &std::hashmap::HashMap<u64, ~[~str]>,
                  root_name : &str,
                  discriminantOffset : u32,
                  fields : &[schema_capnp::Field::Reader],
                  is_reader : bool)
    -> (FormattedText, FormattedText) {

    use schema_capnp::*;

    let mut getter_interior = ~[];

    let mut interior = ~[];
    let mut enum_interior = ~[];

    let doffset = discriminantOffset as uint;

    let mut requiresSelfVar = false;

    for field in fields.iter() {

        let dvalue = field.get_discriminant_value() as uint;

        let fieldName = field.get_name();
        let enumerantName = capitalize_first_letter(fieldName);

        let qual_enumerant_name = if is_reader {
            enumerantName.clone()
        } else {
            format!("Which::{}", enumerantName)
        };

        let (ty, get) = getter_text(nodeMap, scopeMap, field, is_reader);

        enum_interior.push(Line(format!("{}({}),", enumerantName, ty)));

        getter_interior.push(Branch(~[
                    Line(format!("{} => \\{", dvalue)),
                    Indent(~Line(format!("return Some({}(", qual_enumerant_name))),
                    Indent(~Indent(~get)),
                    Indent(~Line(~"));")),
                    Line(~"}")
                ]));

        match field.which() {
            Some(Field::Group(_)) => requiresSelfVar = true,
            Some(Field::Slot(regField)) => {
                match regField.get_type().which() {
                    Some(Type::Text(())) | Some(Type::Data(())) |
                    Some(Type::List(_)) | Some(Type::Struct(_)) |
                    Some(Type::AnyPointer(())) => requiresSelfVar = true,
                    _ => ()
                }
            }
            _ => ()
        }
    }

    let lifetime_suffix = if requiresSelfVar { "<'a>" } else { "" };
    let enum_name = format!("Which{}{}", if is_reader { "Reader" } else { "Builder" }, lifetime_suffix);

    getter_interior.push(Line(~"_ => return None"));

    interior.push(
        Branch(~[Line(format!("pub enum {} \\{", enum_name)),
                 Indent(~Branch(enum_interior)),
                 Line(~"}")]));


    let result = if is_reader {
        Branch(interior)
    } else {
        Branch(~[Line(~"pub mod Which {"),
                 Indent(~generate_import_statements(root_name)),
                 BlankLine,
                 Indent(~Branch(interior)),
                 Line(~"}")])
    };

    let field_name = if is_reader { "reader" } else { "builder" };

    let which_return_type = if is_reader {
        format!("Option<{}>", enum_name)
    } else {
        format!("Option<Which::{}>", enum_name)
    };

    let getter_result =
        Branch(~[Line(~"#[inline]"),
                 Line(format!("pub fn which(&self) -> {} \\{",
                              which_return_type)),
                 Indent(~Branch(~[
                     Line(format!("match self.{}.get_data_field::<u16>({}) \\{", field_name, doffset)),
                     Indent(~Branch(getter_interior)),
                     Line(~"}")
                 ])),
                 Line(~"}")]);

    // TODO set_which() for builders?

    return (result, getter_result);
}


fn generate_node(nodeMap : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                 scopeMap : &std::hashmap::HashMap<u64, ~[~str]>,
                 rootName : &str,
                 node_id : u64) -> FormattedText {
    use schema_capnp::*;

    let mut output: ~[FormattedText] = ~[];
    let mut nested_output: ~[FormattedText] = ~[];

    let nodeReader = nodeMap.get(&node_id);
    let nestedNodes = nodeReader.get_nested_nodes();
    for ii in range(0, nestedNodes.size()) {
        nested_output.push(generate_node(nodeMap, scopeMap, rootName, nestedNodes[ii].get_id()));
    }

    match nodeReader.which() {

        Some(Node::File(())) => {
            output.push(Branch(nested_output));
        }

        Some(Node::Struct(structReader)) => {
            let names = scopeMap.get(&node_id);
            output.push(BlankLine);
            output.push(Line(format!("pub mod {} \\{", *names.last())));

            let mut preamble = ~[];
            let mut builder_members = ~[];
            let mut reader_members = ~[];
            let mut union_fields = ~[];
            let mut which_enums = ~[];

            let dataSize = structReader.get_data_word_count();
            let pointerSize = structReader.get_pointer_count();
            let preferred_list_encoding =
                  match structReader.get_preferred_list_encoding() {
                                Some(e) => e,
                                None => fail!("unsupported list encoding")
                        };
            let isGroup = structReader.get_is_group();
            let discriminantCount = structReader.get_discriminant_count();
            let discriminantOffset = structReader.get_discriminant_offset();

            preamble.push(generate_import_statements(rootName));
            preamble.push(BlankLine);


            if (!isGroup) {
                preamble.push(Line(~"pub static STRUCT_SIZE : layout::StructSize ="));
                preamble.push(
                   Indent(
                      ~Line(
                        format!("layout::StructSize \\{ data : {}, pointers : {}, preferred_list_encoding : layout::{}\\};",
                             dataSize as uint, pointerSize as uint,
                             element_size_str(preferred_list_encoding)))));
                preamble.push(BlankLine);

                preamble.push(BlankLine);
            }

            let fields = structReader.get_fields();
            for ii in range(0, fields.size()) {
                let field = fields[ii];
                let name = field.get_name();
                let styled_name = camel_to_snake_case(name);

                let discriminantValue = field.get_discriminant_value();
                let isUnionField = (discriminantValue != 0xffff);

                if (!isUnionField) {
                    let (ty, get) = getter_text(nodeMap, scopeMap, &field, true);

                    reader_members.push(
                           Branch(~[
                              Line(~"#[inline]"),
                              Line(format!("pub fn get_{}(&self) -> {} \\{", styled_name, ty)),
                              Indent(~get),
                              Line(~"}")
                                    ])
                                        );

                    let (tyB, getB) = getter_text(nodeMap, scopeMap, &field, false);

                    builder_members.push(
                                     Branch(~[
                                              Line(~"#[inline]"),
                                              Line(format!("pub fn get_{}(&self) -> {} \\{", styled_name, tyB)),
                                              Indent(~getB),
                                              Line(~"}")
                                              ])
                                     );


                } else {
                    union_fields.push(field);
                }

                builder_members.push(generate_setter(nodeMap, scopeMap,
                                                    discriminantOffset,
                                                    styled_name, &field));


                match field.which() {
                    Some(Field::Group(group)) => {
                        let text = generate_node(nodeMap, scopeMap, rootName, group.get_type_id());
                        nested_output.push(text);
                    }
                    _ => { }
                }

            }

            if (discriminantCount > 0) {
                let (which_enums1, union_getter) =
                    generate_union(nodeMap, scopeMap, rootName,
                                   discriminantOffset, union_fields, true);
                which_enums.push(which_enums1);
                reader_members.push(union_getter);

                let (which_enums2, union_getter) =
                    generate_union(nodeMap, scopeMap, rootName,
                                   discriminantOffset, union_fields, false);
                which_enums.push(which_enums2);
                builder_members.push(union_getter);
            }

            let builderStructSize =
                if (isGroup) { Branch(~[] ) }
                else {
                  Branch(~[
                       Line(~"impl <'a> layout::HasStructSize for Builder<'a> {"),
                       Indent(~Branch(~[Line(~"#[inline]"),
                                        Line(~"fn struct_size(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }")])),
                       Line(~"}")])
            };

            let accessors =
                ~[Branch(preamble),
                  // TODO figure out how to arrange that this field can be private.
                  Line(~"pub struct Reader<'a> { reader : layout::StructReader<'a> }"),
                  BlankLine,
                  Line(~"impl <'a> layout::FromStructReader<'a> for Reader<'a> {"),
                  Indent(
                    ~Branch(
                        ~[Line(~"fn from_struct_reader(reader: layout::StructReader<'a>) -> Reader<'a> {"),
                          Indent(~Line(~"Reader { reader : reader }")),
                          Line(~"}")])),
                  Line(~"}"),
                  BlankLine,
                  Line(~"impl <'a> Reader<'a> {"),
                  Indent(
                      ~Branch(
                          ~[Line(~"pub fn new<'a>(reader : layout::StructReader<'a>) \
                                                  -> Reader<'a> {"),
                            Indent(~Line(~"Reader { reader : reader }")),
                            Line(~"}")
                            ])),
                  Indent(~Branch(reader_members)),
                  Line(~"}"),
                  BlankLine,
                  Line(~"pub struct Builder<'a> { priv builder : layout::StructBuilder<'a> }"),
                  builderStructSize,
                  Line(~"impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {"),
                  Indent(
                      ~Branch(
                          ~[Line(~"fn from_struct_builder(builder : layout::StructBuilder<'a>) -> Builder<'a> {"),
                            Indent(~Line(~"Builder { builder : builder }")),
                            Line(~"}")
                            ])),
                  Line(~"}"),

                  Line(~"impl <'a> Builder<'a> {"),
                  Indent(
                      ~Branch(
                          ~[Line(~"pub fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {"),
                            Indent(~Line(~"Builder { builder : builder }")),
                            Line(~"}"),
                            BlankLine,
                            Line(~"pub fn as_reader<T>(&self, f : |Reader| -> T) -> T {"),
                            Indent(~Line(~"self.builder.as_reader( |reader| {")),
                            Indent(~Indent(~Line(~"f(Reader::new(reader))"))),
                            Indent(~Line(~"})")),
                            Line(~"}")
                            ])),
                  Indent(~Branch(builder_members)),
                  Line(~"}")];

            output.push(Indent(~Branch(~[Branch(accessors),
                                         Branch(which_enums),
                                         Branch(nested_output)])));
            output.push(Line(~"}"));

        }

        Some(Node::Enum(enumReader)) => {
            let names = scopeMap.get(&node_id);
            output.push(Line(format!("pub mod {} \\{", *names.last())));

            output.push(Indent(~Line(~"use capnp::list::{ToU16};")));
            output.push(BlankLine);

            let mut members = ~[];
            let enumerants = enumReader.get_enumerants();
            for ii in range(0, enumerants.size()) {
                let enumerant = enumerants[ii];
                members.push(
                    Line(format!("{} = {},", capitalize_first_letter(enumerant.get_name()),
                              ii)));
            }

            output.push(Indent(~Branch(~[Line(~"#[repr(u16)]"),
                                         Line(~"#[deriving(FromPrimitive)]"),
                                         Line(~"#[deriving(Eq)]"),
                                         Line(~"pub enum Reader {"),
                                         Indent(~Branch(members)),
                                         Line(~"}")])));
            output.push(
                Indent(
                    ~Branch(
                        ~[Line(~"impl ToU16 for Reader {"),
                          Indent(~Line(~"#[inline]")),
                          Indent(
                            ~Line(~"fn to_u16(self) -> u16 { self as u16 }")),
                          Line(~"}")])));

            output.push(Line(~"}"));
        }

        Some(Node::Interface(_)) => { }

        Some(Node::Const(c)) => {
            let names = scopeMap.get(&node_id);
            let styled_name = camel_to_upper_case(*names.last());

            let (typ, txt) = match (c.get_type().which(), c.get_value().which()) {
                (Some(Type::Void(())), Some(Value::Void(()))) => (~"()", ~"()"),
                (Some(Type::Bool(())), Some(Value::Bool(b))) => (~"bool", b.to_str()),
                (Some(Type::Int8(())), Some(Value::Int8(i))) => (~"i8", i.to_str()),
                (Some(Type::Int16(())), Some(Value::Int16(i))) => (~"i16", i.to_str()),
                (Some(Type::Int32(())), Some(Value::Int32(i))) => (~"i32", i.to_str()),
                (Some(Type::Int64(())), Some(Value::Int64(i))) => (~"i64", i.to_str()),
                (Some(Type::Uint8(())), Some(Value::Uint8(i))) => (~"u8", i.to_str()),
                (Some(Type::Uint16(())), Some(Value::Uint16(i))) => (~"u16", i.to_str()),
                (Some(Type::Uint32(())), Some(Value::Uint32(i))) => (~"u32", i.to_str()),
                (Some(Type::Uint64(())), Some(Value::Uint64(i))) => (~"u64", i.to_str()),

                // float string formatting appears to be a bit broken currently, in Rust.
                (Some(Type::Float32(())), Some(Value::Float32(f))) => (~"f32", format!("{}f32", f.to_str())),
                (Some(Type::Float64(())), Some(Value::Float64(f))) => (~"f64", format!("{}f64", f.to_str())),

                (Some(Type::Text(())), Some(Value::Text(_t))) => { fail!() }
                (Some(Type::Data(())), Some(Value::Data(_d))) => { fail!() }
                (Some(Type::List(_t)), Some(Value::List(_p))) => { fail!() }
                (Some(Type::Struct(_t)), Some(Value::Struct(_p))) => { fail!() }
                (Some(Type::Interface(_t)), Some(Value::Interface)) => { fail!() }
                (Some(Type::AnyPointer(())), Some(Value::AnyPointer(_pr))) => { fail!() }
                _ => { fail!("type does not match value") }
            };

            output.push(
                Line(format!("pub static {} : {} = {};", styled_name, typ, txt)));
        }

        Some(Node::Annotation( annotationReader )) => {
            println("  annotation node:");
            if (annotationReader.get_targets_file()) {
                println("  targets file");
            }
            if (annotationReader.get_targets_const()) {
                println("  targets const");
            }
            // ...
            if (annotationReader.get_targets_annotation()) {
                println("  targets annotation");
            }
        }

        None => ()
    }

    Branch(output)
}


fn main() {
    use std::io::{Writer, File, Truncate, Write};
    use capnp::serialize::*;

    let mut inp = std::io::stdin();

    InputStreamMessageReader::new(&mut inp, message::DEFAULT_READER_OPTIONS, |messageReader| {
        let codeGeneratorRequest : schema_capnp::CodeGeneratorRequest::Reader = messageReader.get_root();

        let mut nodeMap = std::hashmap::HashMap::<u64, schema_capnp::Node::Reader>::new();
        let mut scopeMap = std::hashmap::HashMap::<u64, ~[~str]>::new();

        let nodeListReader = codeGeneratorRequest.get_nodes();

        for ii in range(0, nodeListReader.size()) {
            nodeMap.insert(nodeListReader[ii].get_id(), nodeListReader[ii]);
        }

        let requestedFilesReader = codeGeneratorRequest.get_requested_files();

        for ii in range(0, requestedFilesReader.size()) {

            let requestedFile = requestedFilesReader[ii];
            let id = requestedFile.get_id();
            let name : &str = requestedFile.get_filename();
            println(format!("requested file: {}", name));

            let fileNode = nodeMap.get(&id);
            let displayName = fileNode.get_display_name();

            let mut outputFileName : ~str =
                match displayName.rfind('.') {
                    Some(d) => {
                        displayName.slice_chars(0, d).to_owned()
                    }
                    _ => { fail!("bad file name: {}", displayName) }
                };

            outputFileName.push_str("_capnp");

            let rootName : ~str =
                match outputFileName.rfind('/') {
                Some(s) => outputFileName.slice_chars(s + 1,outputFileName.len()).to_owned(),
                None => outputFileName.as_slice().to_owned()
            };

            outputFileName.push_str(".rs");
            println(outputFileName);

            populate_scope_map(&nodeMap, &mut scopeMap, rootName, id);

            let lines = Branch(~[Line(~"#[allow(unused_imports)];"),
                                 generate_node(&nodeMap, &scopeMap,
                                               rootName, id)]);
            let text = stringify(&lines);

            let path = std::path::Path::new(outputFileName);

            match File::open_mode(&path, Truncate, Write) {
                Some(ref mut writer) => {
                    writer.write(text.as_bytes())
            }
                None => {fail!("could not open file for writing")}
            }
        }
    });
}
