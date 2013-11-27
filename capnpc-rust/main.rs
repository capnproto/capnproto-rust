/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[feature(globs)];
#[feature(macro_rules)];

#[link(name = "capnpc-rust", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnp;

use capnp::*;

pub mod schema_capnp;

fn macros() -> ~str {
~"macro_rules! list_submodule(
    ( $capnp:ident::$($m:ident)::+ ) => (
        pub mod List {
            use capnp;
            use $capnp;

            pub struct Reader<'a> {
                priv reader : capnp::layout::ListReader<'a>
            }

            impl <'a> Reader<'a> {
                pub fn new<'b>(reader : capnp::layout::ListReader<'b>) -> Reader<'b> {
                    Reader { reader : reader }
                }
                pub fn size(&self) -> uint { self.reader.size() }
            }

            impl <'a> Index<uint, $capnp::$($m)::+::Reader<'a>> for Reader<'a> {
                fn index(&self, index : &uint) -> $capnp::$($m)::+::Reader<'a> {
                    $capnp::$($m)::+::Reader::new(self.reader.getStructElement(*index))
                }
            }

            pub struct Builder {
                priv builder : capnp::layout::ListBuilder
            }

            impl Builder {
                pub fn new(builder : capnp::layout::ListBuilder) -> Builder {
                    Builder {builder : builder}
                }
                pub fn size(&self) -> uint { self.builder.size() }
            }

            impl Index<uint, $capnp::$($m)::+::Builder> for Builder {
                fn index(&self, index : &uint) -> $capnp::$($m)::+::Builder {
                    $capnp::$($m)::+::Builder::new(self.builder.getStructElement(*index))
                }
            }
        }
    );
)\n\n"
}

fn elementSizeStr (elementSize : schema_capnp::ElementSize::Reader) -> ~ str {
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

fn elementSize (typ : schema_capnp::Type::Which) -> schema_capnp::ElementSize::Reader {
    use schema_capnp::Type::*;
    use schema_capnp::ElementSize::*;
    match typ {
        Void => Empty,
        Bool => Bit,
        Int8 => Byte,
        Int16 => TwoBytes,
        Int32 => FourBytes,
        Int64 => EightBytes,
        Uint8 => Byte,
        Uint16 => TwoBytes,
        Uint32 => FourBytes,
        Uint64 => EightBytes,
        Float32 => FourBytes,
        Float64 => EightBytes,
        _ => fail!("not primitive")
    }
}

fn primTypeStr (typ : schema_capnp::Type::Which) -> ~str {
    use schema_capnp::Type::*;
    match typ {
        Void => ~"()",
        Bool => ~"bool",
        Int8 => ~"i8",
        Int16 => ~"i16",
        Int32 => ~"i32",
        Int64 => ~"i64",
        Uint8 => ~"u8",
        Uint16 => ~"u16",
        Uint32 => ~"u32",
        Uint64 => ~"u64",
        Float32 => ~"f32",
        Float64 => ~"f64",
        _ => fail!("not primitive")
    }
}

fn camelCaseToAllCaps(s : &str) -> ~str {
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
        assert!(std::char::is_alphanumeric(c), format!("not alphanumeric '{}'", c));
        if (std::char::is_uppercase(c)) {
            result_chars.push('_');
        }
        result_chars.push((c as u8).to_ascii().to_lower().to_char());
    }
    return std::str::from_chars(result_chars);
}

fn capitalizeFirstLetter(s : &str) -> ~str {
    use std::ascii::*;
    let bytes = s.as_bytes();
    let mut result_bytes : ~[u8] = ~[];
    for &b in bytes.iter() {
        result_bytes.push(b);
    }
    result_bytes[0] = result_bytes[0].to_ascii().to_upper().to_byte();
    return std::str::from_utf8(result_bytes);
}

#[test]
fn testCamelCaseToAllCaps() {
    assert_eq!(camelCaseToAllCaps("fooBar"), ~"FOO_BAR");
    assert_eq!(camelCaseToAllCaps("fooBarBaz"), ~"FOO_BAR_BAZ");
    assert_eq!(camelCaseToAllCaps("helloWorld"), ~"HELLO_WORLD");
}

#[test]
fn test_camel_to_snake_case() {
    assert_eq!(camel_to_snake_case("fooBar"), ~"foo_bar");
    assert_eq!(camel_to_snake_case("fooBarBaz"), ~"foo_bar_baz");
    assert_eq!(camel_to_snake_case("helloWorld"), ~"hello_world");
    assert_eq!(camel_to_snake_case("uint32Id"), ~"uint32_id");
}

enum FormattedText {
    Indent(~FormattedText),
    Branch(~[FormattedText]),
    Line(~str),
    BlankLine
}

fn toLines(ft : &FormattedText, indent : uint) -> ~[~str] {
    match *ft {
        Indent (ref ft) => {
            return toLines(*ft, indent + 1);
        }
        Branch (ref fts) => {
            return fts.flat_map(|ft| {toLines(ft, indent)});
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
    let mut result = toLines(ft, 0).connect("\n");
    result.push_str("\n");
    return result;
}

fn appendName (names : &[~str], name : ~str) -> ~[~str] {
    let mut result : ~[~str] = ~[];
    for n in names.iter() {
        result.push(n.to_owned());
    }
    result.push(name);
    return result;
}

//type NodeMap = std::hashmap::HashMap<u64, schema_capnp::Node::Reader>;

fn populateScopeMap(nodeMap : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                    scopeMap : &mut std::hashmap::HashMap<u64, ~[~str]>,
                    rootName : &str,
                    nodeId : u64) {
    let nodeReader = nodeMap.get(&nodeId);

    let nestedNodes = nodeReader.get_nested_nodes();
    for ii in range(0, nestedNodes.size()) {
        let nestedNode = nestedNodes[ii];
        let id = nestedNode.get_id();

        let name = capitalizeFirstLetter(nestedNode.get_name());

        let scopeNames = match scopeMap.find(&nodeId) {
            Some(names) => appendName(*names, name),
            None => ~[rootName.to_owned(), name]
        };
        scopeMap.insert(id, scopeNames);
        populateScopeMap(nodeMap, scopeMap, rootName, id);
    }

    match nodeReader.which() {
        Some(schema_capnp::Node::Struct(structReader)) => {
            let fields = structReader.get_fields();
            for jj in range(0, fields.size()) {
                let field = fields[jj];
                match field.which() {
                    Some(schema_capnp::Field::Group(group)) => {
                        let id = group.get_type_id();
                        let name = capitalizeFirstLetter(field.get_name());
                        let scopeNames = match scopeMap.find(&nodeId) {
                            Some(names) => appendName(*names, name),
                            None => ~[rootName.to_owned(), name]
                        };

                        scopeMap.insert(id, scopeNames);
                        populateScopeMap(nodeMap, scopeMap, rootName, id);
                    }
                    _ => {}
                }
            }
        }
        _ => {  }
    }
}

fn generateImportStatements(rootName : &str) -> FormattedText {
    Branch(~[
        Line(~"use std;"),
        Line(~"use capnp::blob::{Text, Data};"),
        Line(~"use capnp::layout;"),
        Line(~"use capnp::list::{PrimitiveList, ToU16, EnumList};"),
        Line(format!("use {};", rootName))
    ])
}

fn getterText (_nodeMap : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
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
                return (format!("{}::Builder", theMod),
                        Line(format!("{}::Builder::new(self.builder)", theMod)));
            }
        }
        Some(Field::Slot(regField)) => {

            let typ = regField.get_type();
            let offset = regField.get_offset() as uint;
            //    let defaultValue = field.getDefaultValue();

            let member = if (isReader) { "reader" } else { "builder" };
            let module = if (isReader) { "Reader" } else { "Builder" };
            let moduleWithVar = if (isReader) { "Reader<'a>" } else { "Builder" };

            match typ.which() {
                Some(Type::Void) => { return (~"()", Line(~"()"))}
                Some(Type::Bool) => {
                    return (~"bool", Line(format!("self.{}.getBoolField({})",
                                                  member, offset)))
                }
                Some(Type::Int8) => {
                    return (~"i8", Line(format!("self.{}.getDataField::<i8>({})",
                                             member, offset)))
                }
                Some(Type::Int16) => {
                    return (~"i16", Line(format!("self.{}.getDataField::<i16>({})",
                                              member, offset)))
                }
                Some(Type::Int32) => {
                    return (~"i32", Line(format!("self.{}.getDataField::<i32>({})",
                                              member, offset)))
                }
                Some(Type::Int64) => {
                    return (~"i64", Line(format!("self.{}.getDataField::<i64>({})",
                                              member, offset)))
                }
                Some(Type::Uint8) => {
                    return (~"u8", Line(format!("self.{}.getDataField::<u8>({})",
                                             member, offset)))
                }
                Some(Type::Uint16) => {
                    return (~"u16", Line(format!("self.{}.getDataField::<u16>({})",
                                              member, offset)))
                }
                Some(Type::Uint32) => {
                    return (~"u32", Line(format!("self.{}.getDataField::<u32>({})",
                                              member, offset)))
                }
                Some(Type::Uint64) => {
                    return (~"u64", Line(format!("self.{}.getDataField::<u64>({})",
                                              member, offset)))
                }
                Some(Type::Float32) => {
                    return (~"f32", Line(format!("self.{}.getDataField::<f32>({})",
                                              member, offset)))
                }
                Some(Type::Float64) => {
                    return (~"f64", Line(format!("self.{}.getDataField::<f64>({})",
                                              member, offset)))
                }
                Some(Type::Text) => {
                    return (format!("Text::{}", moduleWithVar),
                            Line(format!("self.{}.getTextField({}, \"\")",
                                      member, offset)));
                }
                Some(Type::Data) => {
                    return (~"TODO", Line(~"TODO"))
                }
                Some(Type::List(ot1)) => {
                    match ot1.get_element_type().which() {
                        None => { fail!("unsupported type") }
                        Some(Type::Struct(st)) => {
                            let scope = scopeMap.get(&st.get_type_id());
                            let theMod = scope.connect("::");
                            let fullModuleName = format!("{}::List::{}", theMod, module);
                            return (format!("{}::List::{}", theMod, moduleWithVar),
                                    Line(format!("{}::new(self.{}.getListField({}, {}::STRUCT_SIZE.preferredListEncoding, None))",
                                              fullModuleName, member, offset, theMod))
                                    );
                        }
                        Some(Type::Enum(e)) => {
                            let scope = scopeMap.get(&e.get_type_id());
                            let theMod = scope.connect("::");
                            let fullModuleName = format!("{}::Reader", theMod);
                            let typeArgs =
                                if (isReader) {format!("<'a, {}>", fullModuleName)}
                                else {format!("<{}>", fullModuleName)};
                            return (format!("EnumList::{}{}",module,typeArgs),
                                    Line(format!("EnumList::{}::{}::new(self.{}.getListField({},layout::TWO_BYTES,None))",
                                         module, typeArgs, member, offset)));
                        }
                        Some(Type::List(_)) => {return (~"TODO", Line(~"TODO")) }
                        Some(Type::Text) => {return (~"TODO", Line(~"TODO")) }
                        Some(Type::Data) => {return (~"TODO", Line(~"TODO")) }
                        Some(Type::Interface(_)) => {return (~"TODO", Line(~"TODO")) }
                        Some(Type::Object) => {return (~"TODO", Line(~"TODO")) }
                        Some(primType) => {
                            let typeStr = primTypeStr(primType);
                            let sizeStr = elementSizeStr(elementSize(primType));
                            let typeArgs =
                                if (isReader) {format!("<'a, {}>", typeStr)}
                                else {format!("<{}>", typeStr)};
                            return
                                (format!("PrimitiveList::{}{}", module, typeArgs),
                                 Line(format!("PrimitiveList::{}::{}::new(self.{}.getListField({},layout::{},None))",
                                           module, typeArgs, member, offset, sizeStr)))
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
                            ~[Line(format!("FromPrimitive::from_u16(self.{}.getDataField::<u16>({}))",
                                        member, offset))
                              ]));
                }
                Some(Type::Struct(st)) => {
                    let id = st.get_type_id();
                    let scope = scopeMap.get(&id);
                    let theMod = scope.connect("::");
                    let middleArg = if (isReader) {~""} else {format!("{}::STRUCT_SIZE,", theMod)};
                    return (format!("{}::{}", theMod, moduleWithVar),
                            Line(format!("{}::{}::new(self.{}.getStructField({}, {} None))",
                                      theMod, module, member, offset, middleArg)))
                }
                Some(Type::Interface(_)) => {
                        return (~"TODO", Line(~"TODO"));
                }
                Some(Type::Object) => {
                    return (~"TODO", Line(~"TODO"))
                }
                None => {
                    // XXX should probably silently ignore, instead.
                    fail!("unrecognized type")
                }
            }
        }
    }
}

fn generateSetter(_nodeMap : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                  scopeMap : &std::hashmap::HashMap<u64, ~[~str]>,
                  discriminantOffset : u32,
                  capName : &str,
                  field :&schema_capnp::Field::Reader) -> FormattedText {

    use schema_capnp::*;

    let mut result = ~[];
    result.push(Line(~"#[inline]"));

    let mut interior = ~[];

    let discriminantValue = field.get_discriminant_value();
    if (discriminantValue != 0xffff) {
        interior.push(
            Line(format!("self.builder.setDataField::<u16>({}, {});",
                         discriminantOffset as uint,
                         discriminantValue as uint)));
    }

    match field.which() {
        None => fail!("unrecognized field type"),
        Some(Field::Group(group)) => {
            let scope = scopeMap.get(&group.get_type_id());
            let theMod = scope.connect("::");
            result.push(Line(format!("pub fn init{}(&self) -> {}::Builder \\{",
                                     capName, theMod )));
            // XXX todo: zero out all of the fields.
            interior.push(Line(format!("{}::Builder::new(self.builder)", theMod)));
        }
        Some(Field::Slot(regField)) => {
            let offset = regField.get_offset() as uint;

            match regField.get_type().which() {
                Some(Type::Void) => {
                    result.push(Line(format!("pub fn set{}(&self, _value : ()) \\{",capName)))
                }
                Some(Type::Bool) => {
                    result.push(Line(format!("pub fn set{}(&self, value : bool) \\{", capName)));
                    interior.push(Line(format!("self.builder.setBoolField({}, value);", offset)))
                }
                Some(Type::Int8) => {
                    result.push(Line(format!("pub fn set{}(&self, value : i8) \\{", capName)));
                    interior.push(Line(format!("self.builder.setDataField::<i8>({}, value);", offset)))
                }
                Some(Type::Int16) => {
                    result.push(Line(format!("pub fn set{}(&self, value : i16) \\{",capName)));
                    interior.push(Line(format!("self.builder.setDataField::<i16>({}, value);", offset)))
                }
                Some(Type::Int32) => {
                    result.push(Line(format!("pub fn set{}(&self, value : i32) \\{",capName)));
                    interior.push(Line(format!("self.builder.setDataField::<i32>({}, value);", offset)))
                }
                Some(Type::Int64) => {
                    result.push(Line(format!("pub fn set{}(&self, value : i64) \\{",capName)));
                    interior.push(Line(format!("self.builder.setDataField::<i64>({}, value);", offset)))
                }
                Some(Type::Uint8) => {
                    result.push(Line(format!("pub fn set{}(&self, value : u8) \\{",capName)));
                    interior.push(Line(format!("self.builder.setDataField::<u8>({}, value);", offset)))
                }
                Some(Type::Uint16) => {
                    result.push(Line(format!("pub fn set{}(&self, value : u16) \\{",capName)));
                    interior.push(Line(format!("self.builder.setDataField::<u16>({}, value);", offset)))
                }
                Some(Type::Uint32) => {
                    result.push(Line(format!("pub fn set{}(&self, value : u32) \\{",capName)));
                    interior.push(Line(format!("self.builder.setDataField::<u32>({}, value);", offset)))
                }
                Some(Type::Uint64) => {
                    result.push(Line(format!("pub fn set{}(&self, value : u64) \\{",capName)));
                    interior.push(Line(format!("self.builder.setDataField::<u64>({}, value);", offset)))
                }
                Some(Type::Float32) => {
                    result.push(Line(format!("pub fn set{}(&self, value : f32) \\{",capName)));
                    interior.push(Line(format!("self.builder.setDataField::<f32>({}, value);", offset)))
                }
                Some(Type::Float64) => {
                    result.push(Line(format!("pub fn set{}(&self, value : f64) \\{",capName)));
                    interior.push(Line(format!("self.builder.setDataField::<f64>({}, value);", offset)))
                }
                Some(Type::Text) => {
                    result.push(Line(format!("pub fn set{}(&self, value : &str) \\{",capName)));
                    interior.push(Line(format!("self.builder.setTextField({}, value);", offset)))
                }
                Some(Type::Data) => { return BlankLine }
                Some(Type::List(ot1)) => {
                    match ot1.get_element_type().which() {
                        None => fail!("unsupported type"),
                        Some(t1) => {
                            let returnType =
                                match t1 {
                                Type::Void | Type::Bool | Type::Int8 |
                                    Type::Int16 | Type::Int32 | Type::Int64 |
                                    Type::Uint8 | Type::Uint16 | Type::Uint32 |
                                    Type::Uint64 | Type::Float32 | Type::Float64 => {

                                    let typeStr = primTypeStr(t1);
                                    let sizeStr = elementSizeStr(elementSize(t1));

                                    interior.push(Line(format!("PrimitiveList::Builder::<{}>::new(",
                                                            typeStr)));
                                    interior.push(
                                        Indent(~Line(format!("self.builder.initListField({},layout::{},size)",
                                                          offset, sizeStr))));
                                        interior.push(Line(~")"));
                                    format!("PrimitiveList::Builder<{}>", typeStr)
                                }
                                Type::Enum(e) => {
                                    let id = e.get_type_id();
                                    let scope = scopeMap.get(&id);
                                    let theMod = scope.connect("::");
                                    let typeStr = format!("{}::Reader", theMod);
                                    interior.push(Line(format!("EnumList::Builder::<{}>::new(",
                                                            typeStr)));
                                    interior.push(
                                        Indent(
                                            ~Line(
                                                format!("self.builder.initListField({},layout::TWO_BYTES,size)",
                                                     offset))));
                                    interior.push(Line(~")"));
                                    format!("EnumList::Builder<{}>", typeStr)
                                }
                                Type::Struct(st) => {
                                    let id = st.get_type_id();
                                    let scope = scopeMap.get(&id);
                                    let theMod = scope.connect("::");

                                    interior.push(Line(format!("{}::List::Builder::new(", theMod)));
                                    interior.push(
                                       Indent(
                                          ~Line(
                                             format!("self.builder.initStructListField({}, size, {}::STRUCT_SIZE))",
                                                  offset, theMod))));
                                    format!("{}::List::Builder", theMod)
                                }
                                _ => { ~"" }
                            };
                            result.push(Line(format!("pub fn init{}(&self, size : uint) -> {} \\{",
                                                  capName, returnType)))
                       }
                    }
                }
                Some(Type::Enum(e)) => {
                    let id = e.get_type_id();
                    let scope = scopeMap.get(&id);
                    let theMod = scope.connect("::");
                    result.push(Line(format!("pub fn set{}(&self, value : {}::Reader) \\{",
                                          capName, theMod)));
                    interior.push(
                                  Line(format!("self.builder.setDataField::<u16>({}, value as u16)",
                                            offset)));
                }
                Some(Type::Struct(st)) => {
                    let id = st.get_type_id();
                    let scope = scopeMap.get(&id);
                    let theMod = scope.connect("::");
                    result.push(Line(format!("pub fn init{}(&self) -> {}::Builder \\{",capName,theMod)));
                    interior.push(
                      Line(format!("{}::Builder::new(self.builder.initStructField({}, {}::STRUCT_SIZE))",
                                theMod, offset, theMod)));
                }
                Some(Type::Interface(_)) => {
                    return BlankLine
                }
                Some(Type::Object) => {
                    return BlankLine
                }
                None => {return BlankLine}
            }

        }
    }

    result.push(Indent(~Branch(interior)));
    result.push(Line(~"}"));
    return Branch(result);
}


// return (the 'Which' module, the 'which()' accessor)
fn generateUnion(nodeMap : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                 scopeMap : &std::hashmap::HashMap<u64, ~[~str]>,
                 discriminantOffset : u32,
                 fields : &[schema_capnp::Field::Reader])
    -> (FormattedText, FormattedText) {

    use schema_capnp::*;

    let mut result = ~[];

    let mut getter_interior = ~[];

    let mut interior = ~[];
    let mut reader_interior = ~[];

    let doffset = discriminantOffset as uint;

    let mut requiresSelfVar = false;

    for field in fields.iter() {

        let dvalue = field.get_discriminant_value() as uint;

        let fieldName = field.get_name();
        let enumerantName = capitalizeFirstLetter(fieldName);

        let (ty, get) = getterText(nodeMap, scopeMap, field, true);

        reader_interior.push(Line(format!("{}({}),", enumerantName, ty)));

        getter_interior.push(Branch(~[
                    Line(format!("{} => \\{", dvalue)),
                    Indent(~Line(format!("return Some({}(", enumerantName))),
                    Indent(~Indent(~get)),
                    Indent(~Line(~"));")),
                    Line(~"}")
                ]));

        match field.which() {
            Some(Field::Group(_)) => requiresSelfVar = true,
            Some(Field::Slot(regField)) => {
                match regField.get_type().which() {
                    Some(Type::Text) | Some(Type::Data) |
                    Some(Type::List(_)) | Some(Type::Struct(_)) |
                    Some(Type::Object) => requiresSelfVar = true,
                    _ => ()
                }
            }
            _ => ()
        }
    }

    let readerString = if (requiresSelfVar) {"Which<'a>"} else {"Which"};

    getter_interior.push(Line(~"_ => return None"));

    interior.push(
        Branch(~[Line(format!("pub enum {} \\{", readerString)),
                 Indent(~Branch(reader_interior)),
                 Line(~"}")]));


    result.push(Branch(interior));

    let getter_result =
        Branch(~[Line(~"#[inline]"),
                 Line(format!("pub fn which(&self) -> Option<{} > \\{",
                           readerString)),
                 Indent(~Branch(~[
                     Line(format!("match self.reader.getDataField::<u16>({}) \\{", doffset)),
                     Indent(~Branch(getter_interior)),
                     Line(~"}")
                 ])),
                 Line(~"}")]);

    return (Branch(result), getter_result);
}


fn generateNode(nodeMap : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                scopeMap : &std::hashmap::HashMap<u64, ~[~str]>,
                rootName : &str,
                nodeId : u64) -> FormattedText {
    use schema_capnp::*;

    let mut output: ~[FormattedText] = ~[];
    let mut nested_output: ~[FormattedText] = ~[];

    let nodeReader = nodeMap.get(&nodeId);
    let nestedNodes = nodeReader.get_nested_nodes();
    for ii in range(0, nestedNodes.size()) {
        nested_output.push(generateNode(nodeMap, scopeMap, rootName, nestedNodes[ii].get_id()));
    }

    match nodeReader.which() {

        Some(Node::File(())) => {
            output.push(Branch(nested_output));
        }

        Some(Node::Struct(structReader)) => {
            let names = scopeMap.get(&nodeId);
            output.push(BlankLine);

            output.push(Line(~"#[allow(unused_imports)]"));
            output.push(Line(format!("pub mod {} \\{", *names.last())));

            let mut preamble = ~[];
            let mut builder_members = ~[];
            let mut reader_members = ~[];
            let mut which_mod = ~[];
            let mut union_fields = ~[];

            let dataSize = structReader.get_data_word_count();
            let pointerSize = structReader.get_pointer_count();
            let preferredListEncoding =
                  match structReader.get_preferred_list_encoding() {
                                Some(e) => e,
                                None => fail!("unsupported list encoding")
                        };
            let isGroup = structReader.get_is_group();
            let discriminantCount = structReader.get_discriminant_count();
            let discriminantOffset = structReader.get_discriminant_offset();

            preamble.push(generateImportStatements(rootName));
            preamble.push(BlankLine);


            if (!isGroup) {
                preamble.push(Line(~"pub static STRUCT_SIZE : layout::StructSize ="));
                preamble.push(
                   Indent(
                      ~Line(
                        format!("layout::StructSize \\{ data : {}, pointers : {}, preferredListEncoding : layout::{}\\};",
                             dataSize as uint, pointerSize as uint,
                             elementSizeStr(preferredListEncoding)))));
                preamble.push(BlankLine);

                preamble.push(Line(format!("list_submodule!({})",
                                        scopeMap.get(&nodeId).connect("::"))));
                preamble.push(BlankLine);
            }

            let fields = structReader.get_fields();
            for ii in range(0, fields.size()) {
                let field = fields[ii];
                let name = field.get_name();
                let capName = capitalizeFirstLetter(name);

                let discriminantValue = field.get_discriminant_value();
                let isUnionField = (discriminantValue != 0xffff);

                if (!isUnionField) {
                    let (ty, get) = getterText(nodeMap, scopeMap, &field, true);

                    reader_members.push(
                           Branch(~[
                              Line(~"#[inline]"),
                              Line(format!("pub fn get{}(&self) -> {} \\{", capName, ty)),
                              Indent(~get),
                              Line(~"}")
                                    ])
                                        );

                    let (tyB, getB) = getterText(nodeMap, scopeMap, &field, false);

                    builder_members.push(
                                     Branch(~[
                                              Line(~"#[inline]"),
                                              Line(format!("pub fn get{}(&self) -> {} \\{", capName, tyB)),
                                              Indent(~getB),
                                              Line(~"}")
                                              ])
                                     );


                } else {
                    union_fields.push(field);
                }

                builder_members.push(generateSetter(nodeMap, scopeMap,
                                                    discriminantOffset,
                                                    capName, &field));


                match field.which() {
                    Some(Field::Group(group)) => {
                        let text = generateNode(nodeMap, scopeMap, rootName, group.get_type_id());
                        nested_output.push(text);
                    }
                    _ => { }
                }

            }

            if (discriminantCount > 0) {
                let (union_mod, union_getter) =
                    generateUnion(nodeMap, scopeMap,
                                  discriminantOffset, union_fields);
                which_mod.push(union_mod);
                reader_members.push(union_getter);
            }

            let builderStructSize =
                if (isGroup) { Branch(~[] ) }
                else {
                  Branch(~[
                       Line(~"impl layout::HasStructSize for Builder {"),
                       Indent(~Branch(~[Line(~"#[inline]"),
                                        Line(~"fn structSize(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }")])),
                       Line(~"}")])
            };

            let accessors =
                ~[Branch(preamble),
                  Line(~"pub struct Reader<'a> { priv reader : layout::StructReader<'a> }"),
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
                  Line(~"pub struct Builder { priv builder : layout::StructBuilder }"),
                  builderStructSize,
                  Line(~"impl layout::FromStructBuilder for Builder {"),
                  Indent(
                      ~Branch(
                          ~[Line(~"fn fromStructBuilder(builder : layout::StructBuilder) -> Builder {"),
                            Indent(~Line(~"Builder { builder : builder }")),
                            Line(~"}")
                            ])),
                  Line(~"}"),

                  Line(~"impl Builder {"),
                  Indent(
                      ~Branch(
                          ~[Line(~"pub fn new(builder : layout::StructBuilder) -> Builder {"),
                            Indent(~Line(~"Builder { builder : builder }")),
                            Line(~"}"),
                            BlankLine,
                            Line(~"pub fn asReader<T>(&self, f : |Reader| -> T) -> T {"),
                            Indent(~Line(~"self.builder.asReader( |reader| {")),
                            Indent(~Indent(~Line(~"f(Reader::new(reader))"))),
                            Indent(~Line(~"})")),
                            Line(~"}")
                            ])),
                  Indent(~Branch(builder_members)),
                  Line(~"}")];

            output.push(Indent(~Branch(~[Branch(accessors),
                                         Branch(which_mod),
                                         Branch(nested_output)])));
            output.push(Line(~"}"));

        }

        Some(Node::Enum(enumReader)) => {
            let names = scopeMap.get(&nodeId);
            output.push(Line(format!("pub mod {} \\{", *names.last())));

            output.push(Indent(~Line(~"use capnp::list::{ToU16};")));

            let mut members = ~[];
            let enumerants = enumReader.get_enumerants();
            for ii in range(0, enumerants.size()) {
                let enumerant = enumerants[ii];
                members.push(
                    Line(format!("{} = {},", capitalizeFirstLetter(enumerant.get_name()),
                              ii)));
            }

            output.push(Indent(~Branch(~[Line(~"#[repr(u16)]"),
                                         Line(~"#[deriving(FromPrimitive)]"),
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

        Some(Node::Const(_)) => { }

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
        let structReader = messageReader.getRoot();

        let codeGeneratorRequest =
            schema_capnp::CodeGeneratorRequest::Reader::new(structReader);

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

            populateScopeMap(&nodeMap, &mut scopeMap, rootName, id);

            let text = stringify(&generateNode(&nodeMap, &scopeMap,
                                               rootName, id));

            let macros_text = macros();

            let path = std::path::Path::new(outputFileName);

            match File::open_mode(&path, Truncate, Write) {
                Some(ref mut writer) => {
                    writer.write(macros_text.as_bytes());
                    writer.write(text.as_bytes())
            }
                None => {fail!("could not open file for writing")}
            }
        }
    });
}
