/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[feature(globs)];
#[feature(macro_rules)];

#[link(name = "capnpc-rust", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnprust;

use capnprust::*;

pub mod schema_capnp;

fn macros() -> ~str {
~"macro_rules! list_submodule(
    ( $capnp:ident::$($m:ident)::+ ) => (
        pub mod List {
            use capnprust;
            use $capnp;

            pub struct Reader<'self> {
                reader : capnprust::layout::ListReader<'self>
            }

            impl <'self> Reader<'self> {
                pub fn new<'a>(reader : capnprust::layout::ListReader<'a>) -> Reader<'a> {
                    Reader { reader : reader }
                }
                pub fn size(&self) -> uint { self.reader.size() }
                pub fn get(&self, index : uint) -> $capnp::$($m)::+::Reader<'self> {
                    $capnp::$($m)::+::Reader::new(self.reader.getStructElement(index))
                }
            }

            pub struct Builder {
                builder : capnprust::layout::ListBuilder
            }

            impl Builder {
                pub fn new(builder : capnprust::layout::ListBuilder) -> Builder {
                    Builder {builder : builder}
                }
                pub fn size(&self) -> uint { self.builder.size() }
                pub fn get(&self, index : uint) -> $capnp::$($m)::+::Builder {
                    $capnp::$($m)::+::Builder::new(self.builder.getStructElement(index))
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
    let bytes = s.as_bytes();
    let mut result_bytes : ~[u8] = ~[];
    for &b in bytes.iter() {

        // strings will be null-terminated
        if (b != 0) {
            let c = b as char;
            assert!(std::char::is_alphanumeric(c), format!("not alphanumeric '{}'", c));
            if (std::char::is_uppercase(c)) {
                result_bytes.push('_' as u8);
            }

            let b1 = b.to_ascii().to_upper().to_byte();

            result_bytes.push(b1);
        }
    }
    return std::str::from_utf8(result_bytes);
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
            return do fts.flat_map |ft| {toLines(ft, indent)};
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

    let nestedNodes = nodeReader.getNestedNodes();
    for ii in range(0, nestedNodes.size()) {
        let nestedNode = nestedNodes.get(ii);
        let id = nestedNode.getId();

        let name = capitalizeFirstLetter(nestedNode.getName());

        let scopeNames = match scopeMap.find(&nodeId) {
            Some(names) => appendName(*names, name),
            None => ~[rootName.to_owned(), name]
        };
        scopeMap.insert(id, scopeNames);
        populateScopeMap(nodeMap, scopeMap, rootName, id);
    }

    match nodeReader.which() {
        Some(schema_capnp::Node::Struct(structReader)) => {
            let fields = structReader.getFields();
            for jj in range(0, fields.size()) {
                let field = fields.get(jj);
                match field.which() {
                    Some(schema_capnp::Field::Group(group)) => {
                        let id = group.getTypeId();
                        let name = capitalizeFirstLetter(field.getName());
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
        Line(~"use capnprust::blob::{Text, Data};"),
        Line(~"use capnprust::layout;"),
        Line(~"use capnprust::list::{PrimitiveList, ToU16, EnumList};"),
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
            let id = group.getTypeId();
            let scope = scopeMap.get(&id);
            let theMod = scope.connect("::");
            if (isReader) {
                return (format!("{}::Reader<'self>", theMod),
                        Line(format!("{}::Reader::new(self._reader)", theMod)));
            } else {
                return (format!("{}::Builder", theMod),
                        Line(format!("{}::Builder::new(self._builder)", theMod)));
            }
        }
        Some(Field::Slot(regField)) => {

            let typ = regField.getType();
            let offset = regField.getOffset() as uint;
            //    let defaultValue = field.getDefaultValue();

            let member = if (isReader) { "_reader" } else { "_builder" };
            let module = if (isReader) { "Reader" } else { "Builder" };
            let moduleWithVar = if (isReader) { "Reader<'self>" } else { "Builder" };

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
                    match ot1.getElementType().which() {
                        None => { fail!("unsupported type") }
                        Some(Type::Struct(st)) => {
                            let id = st.getTypeId();
                            let scope = scopeMap.get(&id);
                            let theMod = scope.connect("::");
                            let fullModuleName = format!("{}::List::{}", theMod, module);
                            return (format!("{}::List::{}", theMod, moduleWithVar),
                                    Line(format!("{}::new(self.{}.getListField({}, {}::STRUCT_SIZE.preferredListEncoding, None))",
                                              fullModuleName, member, offset, theMod))
                                    );
                        }
                        Some(Type::Enum(e)) => {
                            let id = e.getTypeId();
                            let scope = scopeMap.get(&id);
                            let theMod = scope.connect("::");
                            let fullModuleName = format!("{}::Reader", theMod);
                            let typeArgs =
                                if (isReader) {format!("<'self, {}>", fullModuleName)}
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
                                if (isReader) {format!("<'self, {}>", typeStr)}
                                else {format!("<{}>", typeStr)};
                            return
                                (format!("PrimitiveList::{}{}", module, typeArgs),
                                 Line(format!("PrimitiveList::{}::{}::new(self.{}.getListField({},layout::{},None))",
                                           module, typeArgs, member, offset, sizeStr)))
                        }
                    }
                }
                Some(Type::Enum(en)) => {
                    let id = en.getTypeId();
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
                    let id = st.getTypeId();
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

    let discriminantValue = field.getDiscriminantValue();
    if (discriminantValue != 0xffff) {
            interior.push(
                      Line(format!("self._builder.setDataField::<u16>({}, {});",
                                discriminantOffset as uint,
                                discriminantValue as uint)));
    }

    match field.which() {
        None => fail!("unrecognized field type"),
        Some(Field::Group(group)) => {
            let id = group.getTypeId();
            let scope = scopeMap.get(&id);
            let theMod = scope.connect("::");
            result.push(Line(format!("pub fn init{}(&self) -> {}::Builder \\{",
                                     capName, theMod )));
            // XXX todo: zero out all of the fields.
            interior.push(Line(format!("{}::Builder::new(self._builder)", theMod)));
        }
        Some(Field::Slot(regField)) => {

            let typ = regField.getType();
            let offset = regField.getOffset() as uint;

            match typ.which() {
                Some(Type::Void) => {
                    result.push(Line(format!("pub fn set{}(&self, _value : ()) \\{",capName)))
                }
                Some(Type::Bool) => {
                    result.push(Line(format!("pub fn set{}(&self, value : bool) \\{", capName)));
                    interior.push(Line(format!("self._builder.setBoolField({}, value);", offset)))
                }
                Some(Type::Int8) => {
                    result.push(Line(format!("pub fn set{}(&self, value : i8) \\{", capName)));
                    interior.push(Line(format!("self._builder.setDataField::<i8>({}, value);", offset)))
                }
                Some(Type::Int16) => {
                    result.push(Line(format!("pub fn set{}(&self, value : i16) \\{",capName)));
                    interior.push(Line(format!("self._builder.setDataField::<i16>({}, value);", offset)))
                }
                Some(Type::Int32) => {
                    result.push(Line(format!("pub fn set{}(&self, value : i32) \\{",capName)));
                    interior.push(Line(format!("self._builder.setDataField::<i32>({}, value);", offset)))
                }
                Some(Type::Int64) => {
                    result.push(Line(format!("pub fn set{}(&self, value : i64) \\{",capName)));
                    interior.push(Line(format!("self._builder.setDataField::<i64>({}, value);", offset)))
                }
                Some(Type::Uint8) => {
                    result.push(Line(format!("pub fn set{}(&self, value : u8) \\{",capName)));
                    interior.push(Line(format!("self._builder.setDataField::<u8>({}, value);", offset)))
                }
                Some(Type::Uint16) => {
                    result.push(Line(format!("pub fn set{}(&self, value : u16) \\{",capName)));
                    interior.push(Line(format!("self._builder.setDataField::<u16>({}, value);", offset)))
                }
                Some(Type::Uint32) => {
                    result.push(Line(format!("pub fn set{}(&self, value : u32) \\{",capName)));
                    interior.push(Line(format!("self._builder.setDataField::<u32>({}, value);", offset)))
                }
                Some(Type::Uint64) => {
                    result.push(Line(format!("pub fn set{}(&self, value : u64) \\{",capName)));
                    interior.push(Line(format!("self._builder.setDataField::<u64>({}, value);", offset)))
                }
                Some(Type::Float32) => {
                    result.push(Line(format!("pub fn set{}(&self, value : f32) \\{",capName)));
                    interior.push(Line(format!("self._builder.setDataField::<f32>({}, value);", offset)))
                }
                Some(Type::Float64) => {
                    result.push(Line(format!("pub fn set{}(&self, value : f64) \\{",capName)));
                    interior.push(Line(format!("self._builder.setDataField::<f64>({}, value);", offset)))
                }
                Some(Type::Text) => {
                    result.push(Line(format!("pub fn set{}(&self, value : &str) \\{",capName)));
                    interior.push(Line(format!("self._builder.setTextField({}, value);", offset)))
                }
                Some(Type::Data) => { return BlankLine }
                Some(Type::List(ot1)) => {
                    match ot1.getElementType().which() {
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
                                        Indent(~Line(format!("self._builder.initListField({},layout::{},size)",
                                                          offset, sizeStr))));
                                        interior.push(Line(~")"));
                                    format!("PrimitiveList::Builder<{}>", typeStr)
                                }
                                Type::Enum(e) => {
                                    let id = e.getTypeId();
                                    let scope = scopeMap.get(&id);
                                    let theMod = scope.connect("::");
                                    let typeStr = format!("{}::Reader", theMod);
                                    interior.push(Line(format!("EnumList::Builder::<{}>::new(",
                                                            typeStr)));
                                    interior.push(
                                        Indent(
                                            ~Line(
                                                format!("self._builder.initListField({},layout::TWO_BYTES,size)",
                                                     offset))));
                                    interior.push(Line(~")"));
                                    format!("EnumList::Builder<{}>", typeStr)
                                }
                                Type::Struct(st) => {
                                    let id = st.getTypeId();
                                    let scope = scopeMap.get(&id);
                                    let theMod = scope.connect("::");

                                    interior.push(Line(format!("{}::List::Builder::new(", theMod)));
                                    interior.push(
                                       Indent(
                                          ~Line(
                                             format!("self._builder.initStructListField({}, size, {}::STRUCT_SIZE))",
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
                    let id = e.getTypeId();
                    let scope = scopeMap.get(&id);
                    let theMod = scope.connect("::");
                    result.push(Line(format!("pub fn set{}(&self, value : {}::Reader) \\{",
                                          capName, theMod)));
                    interior.push(
                                  Line(format!("self._builder.setDataField::<u16>({}, value as u16)",
                                            offset)));
                }
                Some(Type::Struct(st)) => {
                    let id = st.getTypeId();
                    let scope = scopeMap.get(&id);
                    let theMod = scope.connect("::");
                    result.push(Line(format!("pub fn init{}(&self) -> {}::Builder \\{",capName,theMod)));
                    interior.push(
                      Line(format!("{}::Builder::new(self._builder.initStructField({}, {}::STRUCT_SIZE))",
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

        let dvalue = field.getDiscriminantValue() as uint;

        let fieldName = field.getName();
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
                match regField.getType().which() {
                    Some(Type::Text) | Some(Type::Data) |
                    Some(Type::List(_)) | Some(Type::Struct(_)) |
                    Some(Type::Object) => requiresSelfVar = true,
                    _ => ()
                }
            }
            _ => ()
        }
    }

    let readerString = if (requiresSelfVar) {"Which<'self>"} else {"Which"};

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
                     Line(format!("match self._reader.getDataField::<u16>({}) \\{", doffset)),
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
    let nestedNodes = nodeReader.getNestedNodes();
    for ii in range(0, nestedNodes.size()) {
        let nestedNode = nestedNodes.get(ii);
        let id = nestedNode.getId();
        let text = generateNode(nodeMap, scopeMap, rootName, id);
        nested_output.push(text);
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

            let dataSize = structReader.getDataWordCount();
            let pointerSize = structReader.getPointerCount();
            let preferredListEncoding =
                  match structReader.getPreferredListEncoding() {
                                Some(e) => e,
                                None => fail!("unsupported list encoding")
                        };
            let isGroup = structReader.getIsGroup();
            let discriminantCount = structReader.getDiscriminantCount();
            let discriminantOffset = structReader.getDiscriminantOffset();

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

            let fields = structReader.getFields();
            for ii in range(0, fields.size()) {
                let field = fields.get(ii);
                let name = field.getName();
                let capName = capitalizeFirstLetter(name);

                let discriminantValue = field.getDiscriminantValue();
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
                        let text = generateNode(nodeMap, scopeMap, rootName, group.getTypeId());
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
                  Line(~"pub struct Reader<'self> { _reader : layout::StructReader<'self> }"),
                  Line(~"impl <'self> Reader<'self> {"),
                  Indent(
                      ~Branch(
                          ~[Line(~"pub fn new<'a>(reader : layout::StructReader<'a>) \
                                                  -> Reader<'a> {"),
                            Indent(~Line(~"Reader { _reader : reader }")),
                            Line(~"}")
                            ])),
                  Indent(~Branch(reader_members)),
                  Line(~"}"),
                  BlankLine,
                  Line(~"pub struct Builder { _builder : layout::StructBuilder }"),
                  builderStructSize,
                  Line(~"impl layout::FromStructBuilder for Builder {"),
                  Indent(
                      ~Branch(
                          ~[Line(~"fn fromStructBuilder(builder : layout::StructBuilder) -> Builder {"),
                            Indent(~Line(~"Builder { _builder : builder }")),
                            Line(~"}")
                            ])),
                  Line(~"}"),

                  Line(~"impl Builder {"),
                  Indent(
                      ~Branch(
                          ~[Line(~"pub fn new(builder : layout::StructBuilder) -> Builder {"),
                            Indent(~Line(~"Builder { _builder : builder }")),
                            Line(~"}"),
                            BlankLine,
                            Line(~"pub fn asReader<T>(&self, f : &fn(Reader) -> T) -> T {"),
                            Indent(~Line(~"do self._builder.asReader |reader| {")),
                            Indent(~Indent(~Line(~"f(Reader::new(reader))"))),
                            Indent(~Line(~"}")),
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

            output.push(Indent(~Line(~"use capnprust::list::{ToU16};")));

            let mut members = ~[];
            let enumerants = enumReader.getEnumerants();
            for ii in range(0, enumerants.size()) {
                let enumerant = enumerants.get(ii);
                members.push(
                    Line(format!("{} = {},", capitalizeFirstLetter(enumerant.getName()),
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
            if (annotationReader.getTargetsFile()) {
                println("  targets file");
            }
            if (annotationReader.getTargetsConst()) {
                println("  targets const");
            }
            // ...
            if (annotationReader.getTargetsAnnotation()) {
                println("  targets annotation");
            }
        }

        None => ()
    }

    Branch(output)
}


fn main() {
    use std::io::{Writer, File, Truncate, Write};
    use capnprust::serialize::*;

    let mut inp = std::io::stdin();

    do InputStreamMessageReader::new(&mut inp, message::DEFAULT_READER_OPTIONS) | messageReader | {
        let structReader = messageReader.getRoot();

        let codeGeneratorRequest =
            schema_capnp::CodeGeneratorRequest::Reader::new(structReader);

        let mut nodeMap = std::hashmap::HashMap::<u64, schema_capnp::Node::Reader>::new();
        let mut scopeMap = std::hashmap::HashMap::<u64, ~[~str]>::new();

        let nodeListReader = codeGeneratorRequest.getNodes();

        for ii in range(0, nodeListReader.size()) {

            let nodeReader = nodeListReader.get(ii);
            let id = nodeReader.getId();
            nodeMap.insert(id, nodeReader);
        }

        let requestedFilesReader = codeGeneratorRequest.getRequestedFiles();

        for ii in range(0, requestedFilesReader.size()) {

            let requestedFile = requestedFilesReader.get(ii);
            let id = requestedFile.getId();
            let name : &str = requestedFile.getFilename();
            println(format!("requested file: {}", name));

            let fileNode = nodeMap.get(&id);
            let displayName = fileNode.getDisplayName();

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
    }
}
