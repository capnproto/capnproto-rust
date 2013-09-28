/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[link(name = "capnpc-rust", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnprust;

use capnprust::*;

pub mod schema_capnp;

fn macros() -> ~str {
~"macro_rules! list_submodule(
    ( $capnp:ident, $($m:ident)::+ ) => (
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
            assert!(std::char::is_alphanumeric(c), fmt!("not alphanumeric '%c'", c));
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
                    nodeId : u64) {
    use schema_capnp::*;
    let nodeReader = nodeMap.get(&nodeId);

    let nestedNodes = nodeReader.getNestedNodes();
    for ii in range(0, nestedNodes.size()) {
        let nestedNode = nestedNodes.get(ii);
        let id = nestedNode.getId();

        let name = capitalizeFirstLetter(nestedNode.getName());

        let scopeNames = {
            if (scopeMap.contains_key(&nodeId)) {
                let names = scopeMap.get(&nodeId);
                appendName(*names, name)
            } else {
                ~[name]
            }
        };
        scopeMap.insert(id, scopeNames);
        populateScopeMap(nodeMap, scopeMap, id);
    }

    match nodeReader.which() {
        Some(Node::Struct(structReader)) => {
            let fields = structReader.getFields();
            for jj in range(0, fields.size()) {
                let field = fields.get(jj);
                match field.which() {
                    Some(Field::Group(group)) => {
                        let id = group.getTypeId();
                        let name = capitalizeFirstLetter(field.getName());
                        let scopeNames = {
                            if (scopeMap.contains_key(&nodeId)) {
                                let names = scopeMap.get(&nodeId);
                                appendName(*names, name)
                            } else {
                                ~[name]
                            }
                        };

                        scopeMap.insert(id, scopeNames);
                        populateScopeMap(nodeMap, scopeMap, id);
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
        Line(~"use capnprust::blob::*;"),
        Line(~"use capnprust::layout::*;"),
        Line(~"use capnprust::list::*;"),
        Line(fmt!("use %s::*;", rootName))
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
                return (fmt!("%s::Reader<'self>", theMod),
                        Line(fmt!("%s::Reader::new(self._reader)", theMod)));
            } else {
                return (fmt!("%s::Builder", theMod),
                        Line(fmt!("%s::Builder::new(self._builder)", theMod)));
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
                    return (~"bool", Line(fmt!("self.%s.getBoolField(%u)",
                                               member, offset)))
                }
                Some(Type::Int8) => {
                    return (~"i8", Line(fmt!("self.%s.getDataField::<i8>(%u)",
                                             member, offset)))
                }
                Some(Type::Int16) => {
                    return (~"i16", Line(fmt!("self.%s.getDataField::<i16>(%u)",
                                              member, offset)))
                }
                Some(Type::Int32) => {
                    return (~"i32", Line(fmt!("self.%s.getDataField::<i32>(%u)",
                                              member, offset)))
                }
                Some(Type::Int64) => {
                    return (~"i64", Line(fmt!("self.%s.getDataField::<i64>(%u)",
                                              member, offset)))
                }
                Some(Type::Uint8) => {
                    return (~"u8", Line(fmt!("self.%s.getDataField::<u8>(%u)",
                                             member, offset)))
                }
                Some(Type::Uint16) => {
                    return (~"u16", Line(fmt!("self.%s.getDataField::<u16>(%u)",
                                              member, offset)))
                }
                Some(Type::Uint32) => {
                    return (~"u32", Line(fmt!("self.%s.getDataField::<u32>(%u)",
                                              member, offset)))
                }
                Some(Type::Uint64) => {
                    return (~"u64", Line(fmt!("self.%s.getDataField::<u64>(%u)",
                                              member, offset)))
                }
                Some(Type::Float32) => {
                    return (~"f32", Line(fmt!("self.%s.getDataField::<f32>(%u)",
                                              member, offset)))
                }
                Some(Type::Float64) => {
                    return (~"f64", Line(fmt!("self.%s.getDataField::<f64>(%u)",
                                              member, offset)))
                }
                Some(Type::Text) => {
                    return (fmt!("Text::%s", moduleWithVar),
                            Line(fmt!("self.%s.getTextField(%u, \"\")",
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
                            let fullModuleName = fmt!("%s::List::%s", theMod, module);
                            return (fmt!("%s::List::%s", theMod, moduleWithVar),
                                    Line(fmt!("%s::new(self.%s.getListField(%u, %s::STRUCT_SIZE.preferredListEncoding, None))",
                                              fullModuleName, member, offset, theMod))
                                    );
                        }
                        Some(Type::Enum(e)) => {
                            let id = e.getTypeId();
                            let scope = scopeMap.get(&id);
                            let theMod = scope.connect("::");
                            let fullModuleName = fmt!("%s::Reader", theMod);
                            let typeArgs =
                                if (isReader) {fmt!("<'self, %s>", fullModuleName)}
                                else {fmt!("<%s>", fullModuleName)};
                            return (fmt!("EnumList::%s%s",module,typeArgs),
                                    Line(fmt!("EnumList::%s::%s::new(self.%s.getListField(%u,TWO_BYTES,None))",
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
                                if (isReader) {fmt!("<'self, %s>", typeStr)}
                                else {fmt!("<%s>", typeStr)};
                            return
                                (fmt!("PrimitiveList::%s%s", module, typeArgs),
                                 Line(fmt!("PrimitiveList::%s::%s::new(self.%s.getListField(%u,%s,None))",
                                           module, typeArgs, member, offset, sizeStr)))
                        }
                    }
                }
                Some(Type::Enum(en)) => {
                    let id = en.getTypeId();
                    let scope = scopeMap.get(&id);
                    let theMod = scope.connect("::");
                    return
                        (fmt!("Option<%s::Reader>", theMod), // Enums don't have builders.
                         Branch(~[
                                Line(fmt!("let result = self.%s.getDataField::<u16>(%u);",
                                          member, offset)),
                                Line(fmt!("let unused_self : Option<%s::Reader> = None;",
                                          theMod)),
                                Line(~"HasMaxEnumerant::cast(unused_self, result)")
                                    ]));
                }
                Some(Type::Struct(st)) => {
                    let id = st.getTypeId();
                    let scope = scopeMap.get(&id);
                    let theMod = scope.connect("::");
                    let middleArg = if (isReader) {~""} else {fmt!("%s::STRUCT_SIZE,", theMod)};
                    return (fmt!("%s::%s", theMod, moduleWithVar),
                            Line(fmt!("%s::%s::new(self.%s.getStructField(%u, %s None))",
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
                      Line(fmt!("self._builder.setDataField::<u16>(%u, %u);",
                                discriminantOffset as uint,
                                discriminantValue as uint)));
    }

    match field.which() {
        None => fail!("unrecognized field type"),
        Some(Field::Group(group)) => {
            let id = group.getTypeId();
            let scope = scopeMap.get(&id);
            let theMod = scope.connect("::");
            result.push(Line(fmt!("pub fn init%s(&self) -> %s::Builder {", capName, theMod )));
            // XXX todo: zero out all of the fields.
            interior.push(Line(fmt!("%s::Builder::new(self._builder)", theMod)));
        }
        Some(Field::Slot(regField)) => {

            let typ = regField.getType();
            let offset = regField.getOffset() as uint;

            match typ.which() {
                Some(Type::Void) => {
                    result.push(Line(fmt!("pub fn set%s(&self, _value : ()) {",capName)))
                }
                Some(Type::Bool) => {
                    result.push(Line(fmt!("pub fn set%s(&self, value : bool) {", capName)));
                    interior.push(Line(fmt!("self._builder.setBoolField(%u, value);", offset)))
                }
                Some(Type::Int8) => {
                    result.push(Line(fmt!("pub fn set%s(&self, value : i8) {", capName)));
                    interior.push(Line(fmt!("self._builder.setDataField::<i8>(%u, value);", offset)))
                }
                Some(Type::Int16) => {
                    result.push(Line(fmt!("pub fn set%s(&self, value : i16) {",capName)));
                    interior.push(Line(fmt!("self._builder.setDataField::<i16>(%u, value);", offset)))
                }
                Some(Type::Int32) => {
                    result.push(Line(fmt!("pub fn set%s(&self, value : i32) {",capName)));
                    interior.push(Line(fmt!("self._builder.setDataField::<i32>(%u, value);", offset)))
                }
                Some(Type::Int64) => {
                    result.push(Line(fmt!("pub fn set%s(&self, value : i64) {",capName)));
                    interior.push(Line(fmt!("self._builder.setDataField::<i64>(%u, value);", offset)))
                }
                Some(Type::Uint8) => {
                    result.push(Line(fmt!("pub fn set%s(&self, value : u8) {",capName)));
                    interior.push(Line(fmt!("self._builder.setDataField::<u8>(%u, value);", offset)))
                }
                Some(Type::Uint16) => {
                    result.push(Line(fmt!("pub fn set%s(&self, value : u16) {",capName)));
                    interior.push(Line(fmt!("self._builder.setDataField::<u16>(%u, value);", offset)))
                }
                Some(Type::Uint32) => {
                    result.push(Line(fmt!("pub fn set%s(&self, value : u32) {",capName)));
                    interior.push(Line(fmt!("self._builder.setDataField::<u32>(%u, value);", offset)))
                }
                Some(Type::Uint64) => {
                    result.push(Line(fmt!("pub fn set%s(&self, value : u64) {",capName)));
                    interior.push(Line(fmt!("self._builder.setDataField::<u64>(%u, value);", offset)))
                }
                Some(Type::Float32) => {
                    result.push(Line(fmt!("pub fn set%s(&self, value : f32) {",capName)));
                    interior.push(Line(fmt!("self._builder.setDataField::<f32>(%u, value);", offset)))
                }
                Some(Type::Float64) => {
                    result.push(Line(fmt!("pub fn set%s(&self, value : f64) {",capName)));
                    interior.push(Line(fmt!("self._builder.setDataField::<f64>(%u, value);", offset)))
                }
                Some(Type::Text) => {
                    result.push(Line(fmt!("pub fn set%s(&self, value : &str) {",capName)));
                    interior.push(Line(fmt!("self._builder.setTextField(%u, value);", offset)))
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

                                    interior.push(Line(fmt!("PrimitiveList::Builder::<%s>::new(",
                                                            typeStr)));
                                    interior.push(
                                        Indent(~Line(fmt!("self._builder.initListField(%u,%s,size)",
                                                          offset, sizeStr))));
                                        interior.push(Line(~")"));
                                    fmt!("PrimitiveList::Builder<%s>", typeStr)
                                }
                                Type::Enum(e) => {
                                    let id = e.getTypeId();
                                    let scope = scopeMap.get(&id);
                                    let theMod = scope.connect("::");
                                    let typeStr = fmt!("%s::Reader", theMod);
                                    interior.push(Line(fmt!("EnumList::Builder::<%s>::new(",
                                                            typeStr)));
                                    interior.push(
                                        Indent(
                                            ~Line(
                                                fmt!("self._builder.initListField(%u,TWO_BYTES,size)",
                                                     offset))));
                                    interior.push(Line(~")"));
                                    fmt!("EnumList::Builder<%s>", typeStr)
                                }
                                Type::Struct(st) => {
                                    let id = st.getTypeId();
                                    let scope = scopeMap.get(&id);
                                    let theMod = scope.connect("::");

                                    interior.push(Line(fmt!("%s::List::Builder::new(", theMod)));
                                    interior.push(
                                       Indent(
                                          ~Line(
                                             fmt!("self._builder.initStructListField(%u, size, %s::STRUCT_SIZE))",
                                                  offset, theMod))));
                                    fmt!("%s::List::Builder", theMod)
                                }
                                _ => { ~"" }
                            };
                            result.push(Line(fmt!("pub fn init%s(&self, size : uint) -> %s {",
                                                  capName, returnType)))
                       }
                    }
                }
                Some(Type::Enum(e)) => {
                    let id = e.getTypeId();
                    let scope = scopeMap.get(&id);
                    let theMod = scope.connect("::");
                    result.push(Line(fmt!("pub fn set%s(&self, value : %s::Reader) {",
                                          capName, theMod)));
                    interior.push(
                                  Line(fmt!("self._builder.setDataField::<u16>(%u, value as u16)",
                                            offset)));
                }
                Some(Type::Struct(st)) => {
                    let id = st.getTypeId();
                    let scope = scopeMap.get(&id);
                    let theMod = scope.connect("::");
                    result.push(Line(fmt!("pub fn init%s(&self) -> %s::Builder {",capName,theMod)));
                    interior.push(
                      Line(fmt!("%s::Builder::new(self._builder.initStructField(%u, %s::STRUCT_SIZE))",
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

        reader_interior.push(Line(fmt!("%s(%s),", enumerantName, ty)));

        getter_interior.push(Branch(~[
                    Line(fmt!("%u => {", dvalue)),
                    Indent(~Line(fmt!("return Some(%s(", enumerantName))),
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
        Branch(~[Line(fmt!("pub enum %s {", readerString)),
                 Indent(~Branch(reader_interior)),
                 Line(~"}")]));


    result.push(Branch(interior));

    let getter_result =
        Branch(~[Line(~"#[inline]"),
                 Line(fmt!("pub fn which(&self) -> Option<%s > {",
                           readerString)),
                 Indent(~Branch(~[
                     Line(fmt!("match self._reader.getDataField::<u16>(%u) {", doffset)),
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
            output.push(Line(fmt!("pub mod %s {", *names.last())));

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
                preamble.push(Line(~"pub static STRUCT_SIZE : StructSize ="));
                preamble.push(
                   Indent(
                      ~Line(
                        fmt!("StructSize { data : %u, pointers : %u, preferredListEncoding : %s};",
                             dataSize as uint, pointerSize as uint,
                             elementSizeStr(preferredListEncoding)))));
                preamble.push(BlankLine);

                preamble.push(Line(fmt!("list_submodule!(%s, %s)",
                                        rootName, scopeMap.get(&nodeId).connect("::"))));
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
                              Line(fmt!("pub fn get%s(&self) -> %s {", capName, ty)),
                              Indent(~get),
                              Line(~"}")
                                    ])
                                        );

                    let (tyB, getB) = getterText(nodeMap, scopeMap, &field, false);

                    builder_members.push(
                                     Branch(~[
                                              Line(~"#[inline]"),
                                              Line(fmt!("pub fn get%s(&self) -> %s {", capName, tyB)),
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
                       Line(~"impl HasStructSize for Builder {"),
                       Indent(~Branch(~[Line(~"#[inline]"),
                                        Line(~"fn structSize(_unused_self : Option<Builder>) -> StructSize { STRUCT_SIZE }")])),
                       Line(~"}")])
            };

            let accessors =
                ~[Branch(preamble),
                  Line(~"pub struct Reader<'self> { _reader : StructReader<'self> }"),
                  Line(~"impl <'self> Reader<'self> {"),
                  Indent(
                      ~Branch(
                          ~[Line(~"pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {"),
                            Indent(~Line(~"Reader { _reader : reader }")),
                            Line(~"}")
                            ])),
                  Indent(~Branch(reader_members)),
                  Line(~"}"),
                  BlankLine,
                  Line(~"pub struct Builder { _builder : StructBuilder }"),
                  builderStructSize,
                  Line(~"impl FromStructBuilder for Builder {"),
                  Indent(
                      ~Branch(
                          ~[Line(~"fn fromStructBuilder(builder : StructBuilder) -> Builder {"),
                            Indent(~Line(~"Builder { _builder : builder }")),
                            Line(~"}")
                            ])),
                  Line(~"}"),

                  Line(~"impl Builder {"),
                  Indent(
                      ~Branch(
                          ~[Line(~"pub fn new(builder : StructBuilder) -> Builder {"),
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
            output.push(Line(fmt!("pub mod %s {", *names.last())));

            output.push(Line(~"use capnprust::list::*;"));

            let mut members = ~[];
            let enumerants = enumReader.getEnumerants();
            for ii in range(0, enumerants.size()) {
                let enumerant = enumerants.get(ii);
                members.push(
                    Line(fmt!("%s = %u,", capitalizeFirstLetter(enumerant.getName()),
                              ii)));
            }

            output.push(Indent(~Branch(~[Line(~"pub enum Reader {"),
                                         Indent(~Branch(members)),
                                         Line(~"}")])));
            output.push(
                Indent(
                    ~Branch(
                        ~[Line(~"impl HasMaxEnumerant for Reader {"),
                          Indent(~Line(~"#[inline]")),
                          Indent(
                            ~Line(
                               fmt!("fn maxEnumerant(_unused_self: Option<Reader>) -> u16 { %u }",
                                    enumerants.size() - 1))),
                          Indent(~Line(~"#[inline]")),
                          Indent(
                            ~Line(~"fn asU16(self) -> u16 { self as u16 }")),
                          Line(~"}")])));

            output.push(Line(~"}"));
        }

        Some(Node::Interface(_)) => { }

        Some(Node::Const(_)) => { }

        Some(Node::Annotation( annotationReader )) => {
            std::io::println("  annotation node:");
            if (annotationReader.getTargetsFile()) {
                std::io::println("  targets file");
            }
            if (annotationReader.getTargetsConst()) {
                std::io::println("  targets const");
            }
            // ...
            if (annotationReader.getTargetsAnnotation()) {
                std::io::println("  targets annotation");
            }
        }

        None => ()
    }

    Branch(output)
}


fn main() {
    use capnprust::serialize::*;

    let inp = std::io::stdin();

    do InputStreamMessageReader::new(inp, message::DEFAULT_READER_OPTIONS) | messageReader | {
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
            std::io::println(fmt!("requested file: %s", name));

            populateScopeMap(&nodeMap, &mut scopeMap, id);

            let fileNode = nodeMap.get(&id);
            let displayName = fileNode.getDisplayName();

            let mut outputFileName : ~str =
                match displayName.rfind('.') {
                    Some(d) => {
                        displayName.slice_chars(0, d).to_owned()
                    }
                    _ => { fail!("bad file name: %s", displayName) }
                };

            outputFileName.push_str("_capnp");

            let rootName : ~str =
                match outputFileName.rfind('/') {
                Some(s) => outputFileName.slice_chars(s + 1,outputFileName.len()).to_owned(),
                None => outputFileName.as_slice().to_owned()
            };

            outputFileName.push_str(".rs");
            std::io::println(outputFileName);

            let text = stringify(&generateNode(&nodeMap, &scopeMap,
                                               rootName, id));

            let macros_text = macros();

            let path = &std::path::Path(outputFileName);
            match std::io::mk_file_writer(path, [std::io::Create, std::io::Truncate]) {
                Ok(writer) => {
                    writer.write(macros_text.as_bytes());
                    writer.write(text.as_bytes())
                }
                Err(msg) => {fail!(msg)}
            }

        }

        0;
    }
}
