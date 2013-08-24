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
        empty => ~"EMPTY",
        bit => ~"BIT",
        byte => ~"BYTE",
        twoBytes => ~"TWO_BYTES",
        fourBytes => ~"FOUR_BYTES",
        eightBytes => ~"EIGHT_BYTES",
        pointer => ~"POINTER",
        inlineComposite => ~"INLINE_COMPOSITE"
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
    return std::str::from_bytes(result_bytes);
}

fn capitalizeFirstLetter(s : &str) -> ~str {
    use std::ascii::*;
    let bytes = s.as_bytes();
    let mut result_bytes : ~[u8] = ~[];
    for &b in bytes.iter() {
        result_bytes.push(b);
    }
    result_bytes[0] = result_bytes[0].to_ascii().to_upper().to_byte();
    return std::str::from_bytes(result_bytes);
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
    let nodeReader = nodeMap.get(&nodeId);

    let nestedNodes = nodeReader.getNestedNodes();
    for ii in range(0, nestedNodes.size()) {
        let nestedNode = nestedNodes.get(ii);
        let id = nestedNode.getId();
        let name = nestedNode.getName().to_owned();

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
        Some(Field::Which::group(id)) => {
            let scope = scopeMap.get(&id);
            let theMod = scope.connect("::");
            if (isReader) {
                return (fmt!("%s::Reader<'self>", theMod),
                        Line(fmt!("%s::Reader::new(self._reader)", theMod)));
            } else {
                return (fmt!("%s::Builder", theMod),
                        Line(fmt!("%s::Builder::new(self._reader)", theMod)));
            }
        }
        Some(Field::Which::nonGroup(regField)) => {

            let typ = regField.getType();
            let offset = regField.getOffset() as uint;
            //    let defaultValue = field.getDefaultValue();

            let member = if (isReader) { "_reader" } else { "_builder" };
            let module = if (isReader) { "Reader" } else { "Builder" };
            let moduleWithVar = if (isReader) { "Reader<'self>" } else { "Builder" };

            match typ.which() {
                Some(Type::Which::void) => { return (~"()", Line(~"()"))}
                Some(Type::Which::bool_) => {
                    return (~"bool", Line(fmt!("self.%s.getBoolField(%u)",
                                               member, offset)))
                }
                Some(Type::Which::int8) => {
                    return (~"i8", Line(fmt!("self.%s.getDataField::<i8>(%u)",
                                             member, offset)))
                }
                Some(Type::Which::int16) => {
                    return (~"i16", Line(fmt!("self.%s.getDataField::<i16>(%u)",
                                              member, offset)))
                }
                Some(Type::Which::int32) => {
                    return (~"i32", Line(fmt!("self.%s.getDataField::<i32>(%u)",
                                              member, offset)))
                }
                Some(Type::Which::int64) => {
                    return (~"i64", Line(fmt!("self.%s.getDataField::<i64>(%u)",
                                              member, offset)))
                }
                Some(Type::Which::uint8) => {
                    return (~"u8", Line(fmt!("self.%s.getDataField::<u8>(%u)",
                                             member, offset)))
                }
                Some(Type::Which::uint16) => {
                    return (~"u16", Line(fmt!("self.%s.getDataField::<u16>(%u)",
                                              member, offset)))
                }
                Some(Type::Which::uint32) => {
                    return (~"u32", Line(fmt!("self.%s.getDataField::<u32>(%u)",
                                              member, offset)))
                }
                Some(Type::Which::uint64) => {
                    return (~"u64", Line(fmt!("self.%s.getDataField::<u64>(%u)",
                                              member, offset)))
                }
                Some(Type::Which::float32) => {
                    return (~"f32", Line(fmt!("self.%s.getDataField::<f32>(%u)",
                                              member, offset)))
                }
                Some(Type::Which::float64) => {
                    return (~"f64", Line(fmt!("self.%s.getDataField::<f64>(%u)",
                                              member, offset)))
                }
                Some(Type::Which::text) => {
                    return (fmt!("Text::%s", moduleWithVar),
                            Line(fmt!("self.%s.getTextField(%u, \"\")",
                                      member, offset)));
                }
                Some(Type::Which::data) => {
                    return (~"TODO", Line(~"TODO"))
                }
                Some(Type::Which::list(t1)) => {
                    match t1.which() {
                        Some(Type::Which::uint64) => {
                            return
                                (fmt!("PrimitiveList::%s", moduleWithVar),
                                 Line(fmt!("PrimitiveList::%s::new(self.%s.getListField(%u,EIGHT_BYTES,None)",
                                           module, member, offset)))
                        }
                        Some(Type::Which::struct_(id)) => {
                            let scope = scopeMap.get(&id);
                            let theMod = scope.connect("::");
                            let fullModuleName = fmt!("%s::List::%s", theMod, module);
                            return (fmt!("%s::List::%s", theMod, moduleWithVar),
                                    Line(fmt!("%s::new(self.%s.getListField(%u, %s::STRUCT_SIZE.preferredListEncoding, None))",
                                              fullModuleName, member, offset, theMod))
                                    );
                        }
                        _ => {return (~"TODO", Line(~"TODO")) }
                    }
                }
                Some(Type::Which::enum_(id)) => {
                    let scope = scopeMap.get(&id);
                    let theMod = scope.connect("::");
                    return
                        (fmt!("Option<%s::Reader>", theMod), // Enums don't have builders.
                         Branch(~[
                                  Line(fmt!("let result = self.%s.getDataField::<u16>(%u) as uint;",
                                            member, offset)),
                                  Line(fmt!("if (result > %s::MAX_ENUMERANT as uint) { None }", theMod)),
                                  Line(~"else { Some(unsafe{std::cast::transmute(result)})}")
                                  ]));
                }
                Some(Type::Which::struct_(id)) => {
                    let scope = scopeMap.get(&id);
                    let theMod = scope.connect("::");
                    let middleArg = if (isReader) {~""} else {fmt!("%s::STRUCT_SIZE,", theMod)};
                    return (fmt!("%s::%s", theMod, moduleWithVar),
                            Line(fmt!("%s::%s::new(self.%s.getStructField(%u, %s None))",
                                      theMod, module, member, offset, middleArg)))
                }
                Some(Type::Which::interface(_)) => {
                        return (~"TODO", Line(~"TODO"));
                }
                Some(Type::Which::object) => {
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
/*
fn generateSetter(_nodeMap : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                  scopeMap : &std::hashmap::HashMap<u64, ~[~str]>,
                  unionOffset : Option<(uint, uint)>,
                  capName : &str,
                  field :&schema_capnp::StructNode::Field::Reader) -> FormattedText {

    use schema_capnp::Type::Body;

    let typ = field.getType();
    let offset = field.getOffset() as uint;

    let mut result = ~[];
    result.push(Line(~"#[inline]"));

    let mut interior = ~[];

    match unionOffset {
        Some((doffset, idx)) => interior.push(
            Line(fmt!("self._builder.setDataField::<u16>(%u, %u);", doffset, idx))),
        None => { }
    }

    match typ.getBody() {
        Body::voidType => {result.push(Line(fmt!("pub fn set%s(&self, _value : ()) {",capName)))}
        Body::boolType => {
            result.push(Line(fmt!("pub fn set%s(&self, value : bool) {",capName)));
            interior.push(Line(fmt!("self._builder.setBoolField(%u, value);", offset)))
        }
        Body::int8Type => {
            result.push(Line(fmt!("pub fn set%s(&self, value : i8) {",capName)));
            interior.push(Line(fmt!("self._builder.setDataField::<i8>(%u, value);", offset)))
        }
        Body::int16Type => {
            result.push(Line(fmt!("pub fn set%s(&self, value : i16) {",capName)));
            interior.push(Line(fmt!("self._builder.setDataField::<i16>(%u, value);", offset)))
        }
        Body::int32Type => {
            result.push(Line(fmt!("pub fn set%s(&self, value : i32) {",capName)));
            interior.push(Line(fmt!("self._builder.setDataField::<i32>(%u, value);", offset)))
        }
        Body::int64Type => {
            result.push(Line(fmt!("pub fn set%s(&self, value : i64) {",capName)));
            interior.push(Line(fmt!("self._builder.setDataField::<i64>(%u, value);", offset)))
        }
        Body::uint8Type => {
            result.push(Line(fmt!("pub fn set%s(&self, value : u8) {",capName)));
            interior.push(Line(fmt!("self._builder.setDataField::<u8>(%u, value);", offset)))
        }
        Body::uint16Type => {
            result.push(Line(fmt!("pub fn set%s(&self, value : u16) {",capName)));
            interior.push(Line(fmt!("self._builder.setDataField::<u16>(%u, value);", offset)))
        }
        Body::uint32Type => {
            result.push(Line(fmt!("pub fn set%s(&self, value : u32) {",capName)));
            interior.push(Line(fmt!("self._builder.setDataField::<u32>(%u, value);", offset)))
        }
        Body::uint64Type => {
            result.push(Line(fmt!("pub fn set%s(&self, value : u64) {",capName)));
            interior.push(Line(fmt!("self._builder.setDataField::<u64>(%u, value);", offset)))
        }
        Body::float32Type => {
            result.push(Line(fmt!("pub fn set%s(&self, value : f32) {",capName)));
            interior.push(Line(fmt!("self._builder.setDataField::<f32>(%u, value);", offset)))
        }
        Body::float64Type => {
            result.push(Line(fmt!("pub fn set%s(&self, value : f64) {",capName)));
            interior.push(Line(fmt!("self._builder.setDataField::<f64>(%u, value);", offset)))
        }
        Body::textType => {
            result.push(Line(fmt!("pub fn set%s(&self, value : &str) {",capName)));
            interior.push(Line(fmt!("self._builder.setTextField(%u, value);", offset)))
        }
        Body::dataType => { return BlankLine }
        Body::listType(t1) => {
            let returnType =
                match t1.getBody() {
                    Body::voidType | Body::boolType | Body::int8Type |
                    Body::int16Type | Body::int32Type | Body::int64Type |
                    Body::uint8Type | Body::uint16Type | Body::uint32Type |
                    Body::uint64Type | Body::float32Type | Body::float64Type => {
                        // TODO
                        ~""
                    }
                    Body::structType(id) => {
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
        Body::enumType(id) => {
            let scope = scopeMap.get(&id);
            let theMod = scope.connect("::");
            result.push(Line(fmt!("pub fn set%s(&self, value : %s::Reader) {",
                                  capName, theMod)));
            interior.push(
                Line(fmt!("self._builder.setDataField::<u16>(%u, value as u16)",
                          offset)));
        }
        Body::structType(id) => {
            let scope = scopeMap.get(&id);
            let theMod = scope.connect("::");
            result.push(Line(fmt!("pub fn init%s(&self) -> %s::Builder {",capName,theMod)));
            interior.push(
                Line(fmt!("%s::Builder::new(self._builder.initStructField(%u, %s::STRUCT_SIZE))",
                          theMod, offset, theMod)));
        }
        Body::interfaceType(_) => {
            return BlankLine
        }
        Body::objectType => {
            return BlankLine
        }
    }

    result.push(Indent(~Branch(interior)));
    result.push(Line(~"}"));
    return Branch(result);
}

// Return (union_mod, union_getter)
fn generateUnion(nodeMap : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
                 scopeMap : &std::hashmap::HashMap<u64, ~[~str]>,
                 rootName : &str,
                 name : &str,
                 union : schema_capnp::StructNode::Union::Reader)
    -> (FormattedText, FormattedText) {

    let mut result = ~[];

    let mut getter_interior = ~[];

    let capitalizedName = capitalizeFirstLetter(name);

    result.push(Line(~"#[allow(unused_imports)]"));
    result.push(Line(fmt!("pub mod %s {", capitalizedName)));
    let mut interior = ~[];
    let mut reader_interior = ~[];
    let mut builder_interior = ~[];

    interior.push(generateImportStatements(rootName));

    builder_interior.push(
        Line(~"pub fn new(builder : StructBuilder) -> Builder { Builder { _builder : builder }}"));

    let doffset = union.getDiscriminantOffset() as uint;

    let members = union.getMembers();
    for ii in range(0, members.size()) {
        let member = members.get(ii);
        let memberName = member.getName();
//        let enumerantName = camelCaseToAllCaps(memberName);
        let enumerantName = memberName;

        match member.getBody() {
            schema_capnp::StructNode::Member::Body::fieldMember(field) => {
                let (ty, get) = getterText(nodeMap, scopeMap, &field, true);

                reader_interior.push(Line(fmt!("%s(%s),",enumerantName, ty)));

                getter_interior.push(Branch(~[
                    Line(fmt!("%u => {", ii)),
                    Indent(~Line(fmt!("return %s::%s(",
                                      capitalizedName, enumerantName))),
                    Indent(~Indent(~get)),
                    Indent(~Line(~");")),
                    Line(~"}")
                ]));

                builder_interior.push(generateSetter(nodeMap, scopeMap, Some((doffset,ii)),
                                                     capitalizeFirstLetter(memberName), &field));

            }
            _ => fail!("impossible")
        }
    }

    getter_interior.push(Line(~"_ => fail!(\"impossible\")"));

    interior.push(
        Branch(~[Line(~"pub enum Reader<'self> {"),
                 Indent(~Branch(reader_interior)),
                 Line(~"}")]));
    interior.push(
        Line(~"pub struct Builder { _builder : StructBuilder }"));
    interior.push(
        Branch(~[Line(~"impl Builder {"),
                 Indent(~Branch(builder_interior)),
                 Line(~"}")]));


    result.push(Indent(~Branch(interior)));
    result.push(Line(~"}"));


    let getter_result =
        Branch(~[Line(~"#[inline]"),
                 Line(fmt!("pub fn get%s(&self) -> %s::Reader<'self> {",
                           capitalizedName, capitalizedName)),
                 Indent(~Branch(~[
                     Line(fmt!("match self._reader.getDataField::<u16>(%u) {", doffset)),
                     Indent(~Branch(getter_interior)),
                     Line(~"}")
                 ])),
                 Line(~"}")]);

    return (Branch(result), getter_result);
}

*/

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

        Some(Node::Which::file_(())) => {
            output.push(Branch(nested_output));
        }

        Some(Node::Which::struct_(structReader)) => {
            let names = scopeMap.get(&nodeId);
            output.push(BlankLine);

            output.push(Line(~"#[allow(unused_imports)]"));
            output.push(Line(fmt!("pub mod %s {", *names.last())));

            let mut preamble = ~[];
            let mut builder_members = ~[];
            let mut reader_members = ~[];
            let mut union_mods = ~[];

            let dataSize = structReader.getDataWordCount();
            let pointerSize = structReader.getPointerCount();
            let preferredListEncoding =
                  match structReader.getPreferredListEncoding() {
                                Some(e) => e,
                                None => fail!("unsupported list encoding")
                        };
            let isGroup = structReader.getIsGroup();
            let discriminantCount = structReader.getDiscriminantCount();

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

                match field.which() {
                    Some(Field::Which::nonGroup(regularField)) => {
                        /*
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

                        builder_members.push(
                            generateSetter(nodeMap, scopeMap, None, capName, &field));
                        */
                    }
                    Some(Field::Which::group(groupField)) => {
                    }
                    None => ()
                }
            }

            if (discriminantCount > 0) {
                    /*
                        let (union_mod, union_getter) =
                            generateUnion(nodeMap, scopeMap, rootName, name, union);
                        union_mods.push(union_mod);
                        reader_members.push(union_getter);

                        builder_members.push(
                            Branch(
                                ~[Line(~"#[inline]"),
                                  Line(fmt!("pub fn get%s(&self) -> %s::Builder {",
                                            capName, capName)),
                                  Indent(
                                      ~Line(
                                          fmt!("%s::Builder::new(self._builder)", capName))),
                                  Line(~"}")]));
                    */
            }

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
                  Line(~"impl HasStructSize for Builder {"),
                  Indent(~Branch(~[Line(~"#[inline]"),
                                   Line(~"fn structSize() -> StructSize { STRUCT_SIZE }")])),
                  Line(~"}"),

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
                                         Branch(union_mods),
                                         Branch(nested_output)])));
            output.push(Line(~"}"));

        }

        Some(Node::Which::enum_(enumReader)) => {
            let names = scopeMap.get(&nodeId);
            output.push(Line(fmt!("pub mod %s {", *names.last())));

            let mut members = ~[];
            let enumerants = enumReader.getEnumerants();
            for ii in range(0, enumerants.size()) {
                let enumerant = enumerants.get(ii);
                members.push(
                    Line(fmt!("%s = %u,", enumerant.getName(), ii)));
            }

            output.push(Indent(~Branch(~[Line(~"pub enum Reader {"),
                                         Indent(~Branch(members)),
                                         Line(~"}")])));
            output.push(Indent(~Line(fmt!("pub static MAX_ENUMERANT : Reader = %s;",
                                          enumerants.get(enumerants.size() - 1).getName()))));
            output.push(Line(~"}"));
        }

        Some(Node::Which::interface(_)) => { }

        Some(Node::Which::const_(_)) => { }

        Some(Node::Which::annotation( annotationReader )) => {
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

        let mut nodeMap = std::hashmap::HashMap::new::<u64, schema_capnp::Node::Reader>();
        let mut scopeMap = std::hashmap::HashMap::new::<u64, ~[~str]>();

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
            std::io::println(displayName);

            let mut outputFileName : ~str =
                match displayName.rfind('.') {
                    Some(d) => {
                        displayName.slice_chars(0, d).to_owned()
                    }
                    _ => { fail!("bad file name: %s", displayName) }
                };

            outputFileName.push_str("_capnp");

            std::io::println(outputFileName);

            let rootName : ~str =
                match outputFileName.rfind('/') {
                Some(s) => outputFileName.slice_chars(s + 1,outputFileName.len()).to_owned(),
                None => outputFileName.as_slice().to_owned()
            };

            outputFileName.push_str(".rs");

            let text = stringify(&generateNode(&nodeMap, &scopeMap,
                                               rootName, id));

            let macros_text = macros();

            let path = &std::path::Path(outputFileName);
            match std::io::mk_file_writer(path, [std::io::Create, std::io::Truncate]) {
                Ok(writer) => {
                    writer.write(macros_text.as_bytes());
                    writer.write(text.as_bytes())
                }
                Err(msg) => {printfln!("ERROR: %s", msg)}
            }

        }

        0;
    }
}
