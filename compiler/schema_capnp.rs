/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

mod macros;

pub mod Node {
    use capnprust::layout::*;
    use capnprust::blob::*;
    use schema_capnp::*;

    pub static STRUCT_SIZE : StructSize = StructSize {data : 5, pointers : 5,
                                                      preferredListEncoding : INLINE_COMPOSITE};

    list_submodule!(schema_capnp, Node)

    pub struct Reader<'self> {
        _reader : StructReader<'self>
    }

    impl <'self> Reader<'self> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ _reader : reader }
        }

        pub fn totalSizeInWords(&self) -> uint {
            self._reader.totalSize() as uint
        }

        pub fn getId(&self) -> u64 {
            self._reader.getDataField::<u64>(0)
        }

        pub fn getDisplayName(&self) -> Text::Reader<'self> {
            self._reader.getTextField(0, "")
        }

        pub fn getDisplayNamePrefixLength(&self) -> u32 {
            self._reader.getDataField::<u32>(2)
        }

        pub fn getScopeId(&self) -> u64 {
            self._reader.getDataField::<u64>(2)
        }

        pub fn getNestedNodes(&self) -> NestedNode::List::Reader<'self> {
            NestedNode::List::Reader::new(self._reader.getListField(1, INLINE_COMPOSITE, None))
        }

        pub fn which(&self) -> Option<Which::Reader<'self>> {
            match self._reader.getDataField::<u16>(6) {
                0 => {
                    return Some(Which::file_(()));
                }
                1 => {
                    return Some(Which::struct_(
                        Struct::Reader::new(self._reader)));
                }
                2 => {
                    return Some(Which::enum_(
                        Enum::Reader::new(self._reader)));
                }
                3 => {
                    return Some(Which::interface(
                        Interface::Reader::new(self._reader)));
                }
                4 => {
                    return Some(Which::const_(
                        Const::Reader::new(self._reader)));
                }
                5 => {
                    return Some(Which::annotation(
                        Annotation::Reader::new(self._reader)));
                }
                _ => return None
            }
        }
    }

    pub struct Builder {
        _builder : StructBuilder
    }

    impl Builder {
        pub fn new(builder : StructBuilder) -> Builder {
            Builder { _builder : builder }
        }
    }

    pub mod Which {
        use capnprust::layout::*;
        use schema_capnp::*;

        pub enum Reader<'self> {
            file_(()),
            struct_(Node::Struct::Reader<'self>),
            enum_(Node::Enum::Reader<'self>),
            interface(Node::Interface::Reader<'self>),
            const_(Node::Const::Reader<'self>),
            annotation(Node::Annotation::Reader<'self>)
        }

        pub struct Builder {
            _builder : StructBuilder
        }

        impl Builder {
            pub fn new(builder : StructBuilder) -> Builder {
                Builder { _builder : builder }
            }
/*
            pub fn initFileNode(&self) -> Node::File::Builder {
                self._builder.setDataField::<u16>(8, 0);
                FileNode::Builder::new(
                    self._builder.initStructField(3, FileNode::STRUCT_SIZE))
            }

            pub fn initStructNode(&self) -> Node::Struct::Builder {
                self._builder.setDataField::<u16>(8, 1);
                StructNode::Builder::new(
                    self._builder.initStructField(3, StructNode::STRUCT_SIZE))
            }
*/
        }
    }

    pub mod Struct {
        use capnprust::layout::*;
        use schema_capnp::*;
        use std;

        pub struct Reader<'self> {
            _reader : StructReader<'self>
        }

        impl <'self> Reader<'self> {

            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ _reader : reader }
            }

            pub fn totalSizeInWords(&self) -> uint {
                self._reader.totalSize() as uint
            }

            pub fn getDataWordCount(&self) -> u16 {
                self._reader.getDataField::<u16>(7)
            }

            pub fn getPointerCount(&self) -> u16 {
                self._reader.getDataField::<u16>(12)
            }

            pub fn getPreferredListEncoding(&self) -> Option<ElementSize::Reader> {
                let result = self._reader.getDataField::<u16>(13) as uint;
                if (result > ElementSize::MAX_ENUMERANT as uint) { None }
                    else { Some( unsafe { std::cast::transmute(result)})}
            }

            pub fn getIsGroup(&self) -> bool {
                self._reader.getBoolField(224)
            }

            pub fn getDiscriminantCount(&self) -> u16 {
                self._reader.getDataField::<u16>(15)
            }

            pub fn getDiscriminantOffset(&self) -> u32 {
                self._reader.getDataField::<u32>(8)
            }

            pub fn getFields(&self) -> Field::List::Reader<'self> {
                Field::List::Reader::new(self._reader.getListField(3, INLINE_COMPOSITE, None))
            }
        }

        pub struct Builder {
            _builder : StructBuilder
        }

        impl Builder {
            pub fn new(builder : StructBuilder) -> Builder {
                Builder { _builder : builder }
            }
        }
    }

    pub mod Enum {
        use schema_capnp::*;
        use capnprust::layout::*;

        pub struct Reader<'self> {
            _reader : StructReader<'self>
        }

        impl <'self> Reader<'self> {

            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ _reader : reader }
            }

            pub fn totalSizeInWords(&self) -> uint {
                self._reader.totalSize() as uint
            }

            pub fn getEnumerants(&self) -> Enumerant::List::Reader<'self> {
                Enumerant::List::Reader::new(
                      self._reader.getListField(3,
                                                Enumerant::STRUCT_SIZE.preferredListEncoding,
                                                None))
            }

        }

        pub struct Builder {
            _builder : StructBuilder
        }

        impl Builder {
            pub fn new(builder : StructBuilder) -> Builder {
                Builder { _builder : builder }
            }
        }

    }

    pub mod Interface {
        use capnprust::layout::*;

        pub struct Reader<'self> {
            _reader : StructReader<'self>
        }

        impl <'self> Reader<'self> {

            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ _reader : reader }
            }

            pub fn totalSizeInWords(&self) -> uint {
                self._reader.totalSize() as uint
            }

            // TODO methods
        }

        pub struct Builder {
            _builder : StructBuilder
        }

        impl Builder {
            pub fn new(builder : StructBuilder) -> Builder {
                Builder { _builder : builder }
            }
        }

    }

    pub mod Const {
        use capnprust::layout::*;
        use schema_capnp::*;

        pub struct Reader<'self> {
            _reader : StructReader<'self>
        }

        impl <'self> Reader<'self> {

            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ _reader : reader }
            }

            pub fn totalSizeInWords(&self) -> uint {
                self._reader.totalSize() as uint
            }

            pub fn getType(&self) -> Type::Reader<'self> {
                Type::Reader::new(self._reader.getStructField(3, None))
            }

            pub fn getValue(&self) -> Value::Reader<'self>{
                Value::Reader::new(self._reader.getStructField(4, None))
            }
        }

        pub struct Builder {
            _builder : StructBuilder
        }

        impl Builder {
            pub fn new(builder : StructBuilder) -> Builder {
                Builder { _builder : builder }
            }
        }
    }

    pub mod Annotation {
        use capnprust::layout::*;
        use schema_capnp::*;

        pub struct Reader<'self> {
            _reader : StructReader<'self>
        }

        impl <'self> Reader<'self> {

            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ _reader : reader }
            }

            pub fn totalSizeInWords(&self) -> uint {
                self._reader.totalSize() as uint
            }

            pub fn getType(&self) -> Type::Reader<'self> {
                Type::Reader::new(self._reader.getStructField(3, None))
            }

            pub fn getTargetsFile(&self) -> bool {
                self._reader.getBoolField(112)
            }

            pub fn getTargetsConst(&self) -> bool {
                self._reader.getBoolField(113)
            }

            pub fn getTargetsEnum(&self) -> bool {
                self._reader.getBoolField(114)
            }

            pub fn getTargetsEnumerant(&self) -> bool {
                self._reader.getBoolField(115)
            }

            pub fn getTargetsStruct(&self) -> bool {
                self._reader.getBoolField(116)
            }

            pub fn getTargetsField(&self) -> bool {
                self._reader.getBoolField(117)
            }

            pub fn getTargetsUnion(&self) -> bool {
                self._reader.getBoolField(118)
            }

            pub fn getTargetsGroup(&self) -> bool {
                self._reader.getBoolField(119)
            }

            pub fn getTargetsInterface(&self) -> bool {
                self._reader.getBoolField(120)
            }

            pub fn getTargetsMethod(&self) -> bool {
                self._reader.getBoolField(121)
            }

            pub fn getTargetsParam(&self) -> bool {
                self._reader.getBoolField(122)
            }

            pub fn getTargetsAnnotation(&self) -> bool {
                self._reader.getBoolField(123)
            }

        }
        pub struct Builder {
            _builder : StructBuilder
        }

        impl Builder {
            pub fn new(builder : StructBuilder) -> Builder {
                Builder { _builder : builder }
            }
        }
    }

    pub mod NestedNode {
        use capnprust::layout::*;
        pub struct Reader<'self> {
            _reader : StructReader<'self>
        }

        impl <'self> Reader<'self> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ _reader : reader }
            }

            pub fn getName(&self) -> &'self str {
                self._reader.getTextField(0, "")
            }

            pub fn getId(&self) -> u64 {
                self._reader.getDataField::<u64>(0)
            }
        }

        pub struct Builder {
            _builder : StructBuilder
        }

        impl Builder {
            pub fn new(builder : StructBuilder) -> Builder {
                Builder { _builder : builder }
            }
        }

        list_submodule!(schema_capnp, Node::NestedNode)
    }

}

pub mod Field {
    use capnprust::layout::*;
    use schema_capnp::*;

    list_submodule!(schema_capnp, Field)

    pub static STRUCT_SIZE : StructSize =
        StructSize {data : 3, pointers : 4,
        preferredListEncoding : INLINE_COMPOSITE};

    pub struct Reader<'self> {
        _reader : StructReader<'self>
    }

    impl <'self> Reader<'self> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ _reader : reader }
        }
    }

    pub struct Builder {
        _builder : StructBuilder
    }

    impl Builder {
        pub fn new(builder : StructBuilder) -> Builder {
            Builder { _builder : builder }
        }
    }

}

pub mod Enumerant {
    use capnprust::layout::*;
    use schema_capnp::*;

    list_submodule!(schema_capnp, Enumerant)

    pub static STRUCT_SIZE : StructSize =
        StructSize {data : 1, pointers : 2,
        preferredListEncoding : INLINE_COMPOSITE};

    pub struct Reader<'self> {
        _reader : StructReader<'self>
    }

    impl <'self> Reader<'self> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ _reader : reader }
        }

        pub fn totalSizeInWords(&self) -> uint {
            self._reader.totalSize() as uint
        }

        pub fn getName(&self) -> &'self str {
            self._reader.getTextField(0, "")
        }

        pub fn getCodeOrder(&self) -> u16 {
            self._reader.getDataField::<u16>(0)
        }

        pub fn getAnnotations(&self) -> Annotation::List::Reader<'self> {
            Annotation::List::Reader::new(
                                          self._reader.getListField(1, Annotation::STRUCT_SIZE.preferredListEncoding,
                                                                    None))
        }
    }

    pub struct Builder {
        _builder : StructBuilder
    }

    impl Builder {
        pub fn new(builder : StructBuilder) -> Builder {
            Builder { _builder : builder }
        }
    }
}

pub mod Method {
    use capnprust::layout::*;

    pub struct Reader<'self> {
        _reader : StructReader<'self>
    }

    impl <'self> Reader<'self> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ _reader : reader }
        }

        pub fn totalSizeInWords(&self) -> uint {
            self._reader.totalSize() as uint
        }
    }

    pub struct Builder {
        _builder : StructBuilder
    }

    impl Builder {
        pub fn new(builder : StructBuilder) -> Builder {
            Builder { _builder : builder }
        }
    }

    list_submodule!(schema_capnp, Method)
}


pub mod Type {
    use capnprust::layout::*;
    use schema_capnp::*;

    pub struct Reader<'self> {
        _reader : StructReader<'self>
    }

    impl <'self> Reader<'self> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ _reader : reader }
        }

        pub fn totalSizeInWords(&self) -> uint {
            self._reader.totalSize() as uint
        }

        pub fn getBody(&self) -> Body::Reader<'self> {
            match self._reader.getDataField::<u16>(0) {
                0 => Body::voidType,
                1 => Body::boolType,
                2 => Body::int8Type,
                3 => Body::int16Type,
                4 => Body::int32Type,
                5 => Body::int64Type,
                6 => Body::uint8Type,
                7 => Body::uint16Type,
                8 => Body::uint32Type,
                9 => Body::uint64Type,
                10 => Body::float32Type,
                11 => Body::float64Type,
                12 => Body::textType,
                13 => Body::dataType,
                14 => {
                    return Body::listType(
                        Type::Reader::new(self._reader.getStructField(0, None)));
                }
                15 => {
                    return Body::enumType(self._reader.getDataField::<u64>(1));
                }
                16 => {
                    return Body::structType(self._reader.getDataField::<u64>(1));
                }
                17 => {
                    return Body::interfaceType(self._reader.getDataField::<u64>(1));
                }
                18 => { Body::objectType }
                _ => fail!("unrecognized discriminant in Type::Body")
            }
        }
    }

    pub struct Builder {
        _builder : StructBuilder
    }

    impl Builder {
        pub fn new(builder : StructBuilder) -> Builder {
            Builder { _builder : builder }
        }
    }


    list_submodule!(schema_capnp, Type)

    pub mod Body {
        use schema_capnp::*;

        pub enum Reader<'self> {
            voidType,
            boolType,
            int8Type,
            int16Type,
            int32Type,
            int64Type,
            uint8Type,
            uint16Type,
            uint32Type,
            uint64Type,
            float32Type,
            float64Type,
            textType,
            dataType,
            listType(Type::Reader<'self>),
            enumType(u64),
            structType(u64),
            interfaceType(u64),
            objectType
        }

    }

}

pub mod Value {
    use capnprust::layout::*;

    pub struct Reader<'self> {
        _reader : StructReader<'self>
    }

    impl <'self> Reader<'self> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ _reader : reader }
        }

        pub fn totalSizeInWords(&self) -> uint {
            self._reader.totalSize() as uint
        }
    }

    pub struct Builder {
        _builder : StructBuilder
    }

    impl Builder {
        pub fn new(builder : StructBuilder) -> Builder {
            Builder { _builder : builder }
        }
    }


    list_submodule!(schema_capnp, Value)

    pub mod Body {
//        use schema_capnp::*;
        use capnprust::blob::*;

        pub enum Reader<'self> {
            voidValue,
            boolValue(bool),
            int8Value(i8),
            int16Value(i16),
            int32Value(i32),
            int64Value(i64),
            uint8Value(u8),
            uint16Value(u16),
            uint32Value(u32),
            uint64Value(u64),
            float32Value(f32),
            float64Value(f32),
            textValue(Text::Reader<'self>),
            dataValue(Data::Reader<'self>),
            listValue, // TODO
            enumValue(u16),
            structValue, // TODO
            interfaceValue,
            objectValue // TODO
        }
    }
}

pub mod Annotation {
    use capnprust::layout::*;
    use schema_capnp::*;

    list_submodule!(schema_capnp, Annotation)
    pub static STRUCT_SIZE : StructSize = StructSize {data : 1, pointers : 1,
                                                      preferredListEncoding : INLINE_COMPOSITE};

    pub struct Reader<'self> {
        _reader : StructReader<'self>
    }

    impl <'self> Reader<'self> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ _reader : reader }
        }

        pub fn totalSizeInWords(&self) -> uint {
            self._reader.totalSize() as uint
        }

        pub fn getId(&self) -> u64 {
            self._reader.getDataField::<u64>(0)
        }

        pub fn getValue(&self) -> Value::Reader<'self> {
            Value::Reader::new(self._reader.getStructField(0, None))
        }
    }

    pub struct Builder {
        _builder : StructBuilder
    }

    impl Builder {
        pub fn new(builder : StructBuilder) -> Builder {
            Builder { _builder : builder }
        }
    }
}

pub mod ElementSize {
    pub enum Reader {
        empty = 0,
        bit = 1,
        byte = 2,
        twoBytes = 3,
        fourBytes = 4,
        eightBytes = 5,
        pointer = 6,
        inlineComposite = 7
    }
    pub static MAX_ENUMERANT : Reader = inlineComposite;
}




pub mod CodeGeneratorRequest {
    use capnprust::layout::*;
    use schema_capnp::*;

    pub static STRUCT_SIZE : StructSize = StructSize {data : 0, pointers : 2,
                                                      preferredListEncoding : INLINE_COMPOSITE};

    list_submodule!(schema_capnp, CodeGeneratorRequest)

    pub struct Reader<'self> {
        _reader : StructReader<'self>
    }

    impl <'self> Reader<'self> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ _reader : reader }
        }

        pub fn getNodes(&self) -> Node::List::Reader<'self> {
            Node::List::Reader::new(self._reader.getListField(0, INLINE_COMPOSITE, None))
        }

        pub fn getRequestedFiles(&self) -> RequestedFile::List::Reader<'self> {
            RequestedFile::List::Reader::new(
                 self._reader.getListField(1,
                                           RequestedFile::STRUCT_SIZE.preferredListEncoding,
                                           None))
        }

    }

    pub struct Builder {
        _builder : StructBuilder
    }

    impl Builder {
        pub fn new(builder : StructBuilder) -> Builder {
            Builder { _builder : builder }
        }

        pub fn initNodes(&self, size : uint) -> Node::List::Builder {
            Node::List::Builder::new(
                self._builder.initStructListField(0, size, Node::STRUCT_SIZE))
        }
    }

    pub mod RequestedFile {
        use capnprust::layout::*;
        use capnprust::blob::*;

        pub static STRUCT_SIZE : StructSize =
            StructSize {data : 1, pointers : 2,
            preferredListEncoding : INLINE_COMPOSITE};

        list_submodule!(schema_capnp, CodeGeneratorRequest::RequestedFile)

        pub struct Reader<'self> {
            _reader : StructReader<'self>
        }

        impl <'self> Reader<'self> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ _reader : reader }
            }

            pub fn getId(&self) -> u64 {
                self._reader.getDataField::<u64>(0)
            }

            pub fn getFilename(&self) -> Text::Reader<'self> {
                self._reader.getTextField(0, "")
            }

            pub fn getImports(&self) -> Import::List::Reader<'self> {
                Import::List::Reader::new(
                 self._reader.getListField(1,
                                           Import::STRUCT_SIZE.preferredListEncoding,
                                           None))
            }
        }

        pub struct Builder {
            _builder : StructBuilder
        }

        impl Builder {
            pub fn new(builder : StructBuilder) -> Builder {
                Builder { _builder : builder }
            }
        }

        pub mod Import {
            use capnprust::layout::*;
            use capnprust::blob::*;

            pub static STRUCT_SIZE : StructSize =
                StructSize {data : 1, pointers : 1,
                preferredListEncoding : INLINE_COMPOSITE};

            list_submodule!(schema_capnp, CodeGeneratorRequest::RequestedFile)

            pub struct Reader<'self> {
                _reader : StructReader<'self>
            }

            impl <'self> Reader<'self> {
                pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                    Reader{ _reader : reader }
                }

                pub fn getId(&self) -> u64 {
                    self._reader.getDataField::<u64>(0)
                }

                pub fn getName(&self) -> Text::Reader<'self> {
                    self._reader.getTextField(0, "")
                }
            }

            pub struct Builder {
                _builder : StructBuilder
            }

            impl Builder {
                pub fn new(builder : StructBuilder) -> Builder {
                    Builder { _builder : builder }
                }
            }

        }

    }

}
