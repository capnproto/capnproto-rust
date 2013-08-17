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

            let b1 = unsafe {
                std::libc::toupper(b as std::libc::c_char) as u8
            };

            result_bytes.push(b1);
        }
    }
    return std::str::from_bytes(result_bytes);
}

fn capitalizeFirstLetter(s : &str) -> ~str {
    let bytes = s.as_bytes();
    let mut result_bytes : ~[u8] = ~[];
    for &b in bytes.iter() {
        result_bytes.push(b);
    }

    result_bytes[0] = unsafe {
        std::libc::toupper(result_bytes[0] as std::libc::c_char) as u8
    };

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
        Line(~"use capnprust::layout::*;"),
        Line(~"use capnprust::list::*;"),
        Line(fmt!("use %s::*;", rootName))
    ])
}

fn getterText (_nodeMap : &std::hashmap::HashMap<u64, schema_capnp::Node::Reader>,
               scopeMap : &std::hashmap::HashMap<u64, ~[~str]>,
               field : &schema_capnp::StructNode::Field::Reader,
                  isReader : bool)
    -> (~str, FormattedText) {

    use schema_capnp::Type::Body;

    let typ = field.getType();
    let offset = field.getOffset() as uint;
//    let defaultValue = field.getDefaultValue();

    let member = if (isReader) { "_reader" } else { "_builder" };
    let module = if (isReader) { "Reader" } else { "Builder" };

    match typ.getBody() {
        Body::voidType => { return (~"()", Line(~"()"))}
        Body::boolType => { return (~"bool", Line(fmt!("self.%s.getBoolField(%u)",
                                                       member, offset))) }
        Body::int8Type => { return (~"i8", Line(fmt!("self.%s.getDataField::<i8>(%u)",
                                                     member, offset))) }
        Body::int16Type => { return (~"i16", Line(fmt!("self.%s.getDataField::<i16>(%u)",
                                                       member, offset))) }
        Body::int32Type => { return (~"i32", Line(fmt!("self.%s.getDataField::<i32>(%u)",
                                                       member, offset))) }
        Body::int64Type => { return (~"i64", Line(fmt!("self.%s.getDataField::<i64>(%u)",
                                                       member, offset))) }
        Body::uint8Type => { return (~"u8", Line(fmt!("self.%s.getDataField::<u8>(%u)",
                                                      member, offset))) }
        Body::uint16Type => { return (~"u16", Line(fmt!("self.%s.getDataField::<u16>(%u)",
                                                        member, offset))) }
        Body::uint32Type => { return (~"u32", Line(fmt!("self.%s.getDataField::<u32>(%u)",
                                                        member, offset))) }
        Body::uint64Type => { return (~"u64", Line(fmt!("self.%s.getDataField::<u64>(%u)",
                                                        member, offset))) }
        Body::float32Type => { return (~"f32", Line(fmt!("self.%s.getDataField::<f32>(%u)",
                                                         member, offset))) }
        Body::float64Type => { return (~"f64", Line(fmt!("self.%s.getDataField::<f64>(%u)",
                                                         member, offset))) }
        Body::textType => { return (~"&'self str", Line(fmt!("self.%s.getTextField(%u, \"\")",
                                                             member, offset))) }
        Body::dataType => {
            return (~"TODO", Line(~"TODO"))
        }
        Body::listType(t1) => {
            match t1.getBody() {
                Body::uint64Type => {
                    return
                        (fmt!("PrimitiveList::%s<'self>", module),
                         Line(fmt!("PrimitiveList::%s::new(self.%s.getListField(%u,EIGHT_BYTES,None)",
                                   module, member, offset)))
                }
                Body::structType(id) => {
                    let scope = scopeMap.get(&id);
                    let theMod = scope.connect("::");
                    let fullModuleName = fmt!("%s::List::%s", theMod, module);
                    return (fmt!("%s<'self>", fullModuleName),
                            Line(fmt!("%s::new(self.%s.getListField(%u, %s::STRUCT_SIZE.preferredListEncoding, None))",
                                      fullModuleName, member, offset, theMod))
                           );
                }
                _ => {return (~"TODO", Line(~"TODO")) }
            }
        }
        Body::enumType(id) => {
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
        Body::structType(id) => {
            let scope = scopeMap.get(&id);
            let theMod = scope.connect("::");
            return (fmt!("%s::%s<'self>", theMod, module),
                    Line(fmt!("%s::%s::new(self.%s.getStructField(%u, None))",
                              theMod, module, member, offset)))
        }
        Body::interfaceType(_) => {
            return (~"TODO", Line(~"TODO"))
        }
        Body::objectType => {
            return (~"TODO", Line(~"TODO"))
        }
    }
}

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
                Line(fmt!("%s::Builder::new(initStructField(%u, %s::STRUCT_SIZE))",
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

    match nodeReader.getBody() {

        Node::Body::fileNode(fileNode) => {
            let imports = fileNode.getImports();
            for ii in range(0, imports.size()) {
                printfln!("  import %s", imports.get(ii).getName());
            }
            output.push(Branch(nested_output));
        }

        Node::Body::structNode(structNode) => {

            let names = scopeMap.get(&nodeId);
            output.push(BlankLine);

            output.push(Line(~"#[allow(unused_imports)]"));
            output.push(Line(fmt!("pub mod %s {", *names.last())));

            let mut preamble = ~[];
            let mut builder_members = ~[];
            let mut reader_members = ~[];
            let mut union_mods = ~[];

            let dataSize = structNode.getDataSectionWordSize();
            let pointerSize = structNode.getPointerSectionSize();
            let preferredListEncoding =
                  match structNode.getPreferredListEncoding() {
                                Some(e) => e,
                                None => fail!("unsupported list encoding")
                        };

            preamble.push(generateImportStatements(rootName));
            preamble.push(BlankLine);
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

            let members = structNode.getMembers();
            for ii in range(0, members.size()) {
                let member = members.get(ii);
                let name = member.getName();
                let capName = capitalizeFirstLetter(name);
                match member.getBody() {
                    StructNode::Member::Body::fieldMember(field) => {
                        let (ty, get) = getterText(nodeMap, scopeMap, &field, true);

                        reader_members.push(
                            Branch(~[
                                Line(~"#[inline]"),
                                Line(fmt!("pub fn get%s(&self) -> %s {", capName, ty)),
                                Indent(~get),
                                Line(~"}")
                            ])
                        );

                        builder_members.push(
                            generateSetter(nodeMap, scopeMap, None, capName, &field));
                    }
                    StructNode::Member::Body::unionMember(union) => {
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
                    }
                }
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
                            Line(~"}")
                            ])),
                  Indent(~Branch(builder_members)),
                  Line(~"}")];

            output.push(Indent(~Branch(~[Branch(accessors),
                                         Branch(union_mods),
                                         Branch(nested_output)])));
            output.push(Line(~"}"));
        }

        Node::Body::enumNode(enumNode) => {
            let names = scopeMap.get(&nodeId);
            output.push(Line(fmt!("pub mod %s {", *names.last())));

            let mut members = ~[];
            let enumerants = enumNode.getEnumerants();
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

        Node::Body::interfaceNode(_) => { }

        Node::Body::constNode(_) => { }

        Node::Body::annotationNode( annotationNode ) => {
            std::io::println("  annotation node:");
            if (annotationNode.getTargetsFile()) {
                std::io::println("  targets file");
            }
            if (annotationNode.getTargetsConst()) {
                std::io::println("  targets const");
            }
            // ...
            if (annotationNode.getTargetsAnnotation()) {
                std::io::println("  targets annotation");
            }
        }
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

            let requestedFileId : u64 = requestedFilesReader.get(ii);
            std::io::println(fmt!("requested file: %x",
                                  requestedFileId as uint));

            populateScopeMap(&nodeMap, &mut scopeMap, requestedFileId);

            let fileNode = nodeMap.get(&requestedFileId);
            let displayName = fileNode.getDisplayName();
            printfln!(displayName);
            let mut rootName : ~str =
                match (displayName.rfind('/'), displayName.rfind('.')) {
                    (Some(s), Some(d)) => {
                        displayName.slice_chars(s + 1, d).to_owned()
                    }
                    (None, Some(d)) => {
                        displayName.slice_chars(0, d).to_owned()
                    }
                    _ => { fail!("bad file name: %s", displayName) }
                };

            rootName.push_str("_capnp");

            let text = stringify(&generateNode(&nodeMap, &scopeMap,
                                               rootName, requestedFileId));
            let macros_text = macros();

            rootName.push_str(".rs");

            let path = &std::path::Path(rootName);
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
