/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

mod macros;

pub mod Node {
    use capnprust::layout::*;
    use capnprust::blob::*;

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

        pub fn which(&self) -> Option<Which<'self>> {
            match self._reader.getDataField::<u16>(6) {
                0 => {
                    return Some(File(()));
                }
                1 => {
                    return Some(Struct(
                        Struct::Reader::new(self._reader)));
                }
                2 => {
                    return Some(Enum(
                        Enum::Reader::new(self._reader)));
                }
                3 => {
                    return Some(Interface(
                        Interface::Reader::new(self._reader)));
                }
                4 => {
                    return Some(Const(
                        Const::Reader::new(self._reader)));
                }
                5 => {
                    return Some(Annotation(
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

    pub enum Which<'self> {
        File(()),
        Struct(Struct::Reader<'self>),
        Enum(Enum::Reader<'self>),
        Interface(Interface::Reader<'self>),
        Const(Const::Reader<'self>),
        Annotation(Annotation::Reader<'self>)
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
    use capnprust::blob::*;
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

        pub fn getName(&self) -> Text::Reader<'self> {
            self._reader.getTextField(0, "")
        }

        pub fn getCodeOrder(&self) -> u16 {
            self._reader.getDataField::<u16>(0)
        }

        pub fn getDiscriminantValue(&self) -> u16 {
            self._reader.getDataFieldMask::<u16>(1, 0xffff)
        }

        pub fn which(&self) -> Option<Which<'self>> {
            match self._reader.getDataField::<u16>(4) {
                0 => {
                    Some(Slot(Slot::Reader::new(self._reader)))
                }
                1 => Some(Group(Group::Reader::new(self._reader))),
                _ => None
            }
        }

        pub fn getOrdinal(&self) -> Ordinal::Reader<'self> {
            Ordinal::Reader::new(self._reader)
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

    pub enum Which<'self> {
        Slot(Field::Slot::Reader<'self>),
        Group(Field::Group::Reader<'self>)
    }

    pub mod Slot {
        use capnprust::layout::*;
        use schema_capnp::*;

        pub struct Reader<'self> {
            _reader : StructReader<'self>
        }

        impl <'self> Reader<'self> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ _reader : reader }
            }

            pub fn getOffset(&self) -> u32 {
                self._reader.getDataField::<u32>(1)
            }

            pub fn getType(&self) -> Type::Reader<'self> {
                Type::Reader::new(self._reader.getStructField(2, None))
            }

            pub fn getDefaultValue(&self) -> Value::Reader<'self> {
                Value::Reader::new(self._reader.getStructField(3, None))
            }
        }
    }

    pub mod Group {
        use capnprust::layout::*;

        pub struct Reader<'self> {
            _reader : StructReader<'self>
        }

        impl <'self> Reader<'self> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ _reader : reader }
            }

            pub fn getTypeId(&self) -> u64 {
                self._reader.getDataField::<u64>(2)
            }
        }
    }


    pub mod Ordinal {
        use capnprust::layout::*;

        pub struct Reader<'self> {
            _reader : StructReader<'self>
        }

        impl <'self> Reader<'self> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ _reader : reader }
            }

            pub fn which(&self) -> Option<Which> {
                match self._reader.getDataField::<u16>(4) {
                    0 => return Some(Implicit(())),
                    1 => return Some(Explicit(self._reader.getDataField::<u16>(6))),
                    _ => return None
                }
            }
        }

        pub enum Which {
            Implicit(()),
            Explicit(u16),
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

        pub fn which(&self) -> Option<Which<'self>> {
            match self._reader.getDataField::<u16>(0) {
                0 => Some(Void),
                1 => Some(Bool),
                2 => Some(Int8),
                3 => Some(Int16),
                4 => Some(Int32),
                5 => Some(Int64),
                6 => Some(Uint8),
                7 => Some(Uint16),
                8 => Some(Uint32),
                9 => Some(Uint64),
                10 => Some(Float32),
                11 => Some(Float64),
                12 => Some(Text),
                13 => Some(Data),
                14 => {
                    return Some(List(List_::Reader::new(self._reader)));
                }
                15 => {
                    return Some(Enum(Enum::Reader::new(self._reader)));
                }
                16 => {
                    return Some(Struct(Struct::Reader::new(self._reader)));
                }
                17 => {
                    return Some(Interface(Interface::Reader::new(self._reader)));
                }
                18 => { return Some(Object); }
                _ => { return None; }
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

    pub enum Which<'self> {
        Void,
        Bool,
        Int8,
        Int16,
        Int32,
        Int64,
        Uint8,
        Uint16,
        Uint32,
        Uint64,
        Float32,
        Float64,
        Text,
        Data,
        List(List_::Reader<'self>),
        Enum(Enum::Reader<'self>),
        Struct(Struct::Reader<'self>),
        Interface(Interface::Reader<'self>),
        Object
    }

    pub mod List_ {
        use capnprust::layout::*;
        use schema_capnp::*;


        pub struct Reader<'self> {
            _reader : StructReader<'self>
        }

        impl <'self> Reader<'self> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ _reader : reader }
            }

            pub fn getElementType(&self) -> Type::Reader<'self> {
                Type::Reader::new(self._reader.getStructField(0, None))
            }
        }
    }

    pub mod Enum {
        use capnprust::layout::*;

        pub struct Reader<'self> {
            _reader : StructReader<'self>
        }

        impl <'self> Reader<'self> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ _reader : reader }
            }

            pub fn getTypeId(&self) -> u64 {
                self._reader.getDataField::<u64>(1)
            }
        }
    }

    pub mod Struct {
        use capnprust::layout::*;

        pub struct Reader<'self> {
            _reader : StructReader<'self>
        }

        impl <'self> Reader<'self> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ _reader : reader }
            }

            pub fn getTypeId(&self) -> u64 {
                self._reader.getDataField::<u64>(1)
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

            pub fn getTypeId(&self) -> u64 {
                self._reader.getDataField::<u64>(1)
            }
        }
    }


}

pub mod Value {
    use capnprust::layout::*;
    use capnprust::blob::*;

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

    pub enum Which<'self> {
        Void,
        Bool(bool),
        Int8(i8),
        Int16(i16),
        Int32(i32),
        Int64(i64),
        Uint8(u8),
        Uint16(u16),
        Uint32(u32),
        Uint64(u64),
        Float32(f32),
        Float64(f32),
        Text(Text::Reader<'self>),
        Data(Data::Reader<'self>),
        List, // TODO
        Enum(u16),
        Struct, // TODO
        Interface,
        Object // TODO
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
        Empty = 0,
        Bit = 1,
        Byte = 2,
        TwoBytes = 3,
        FourBytes = 4,
        EightBytes = 5,
        Pointer = 6,
        InlineComposite = 7
    }
    pub static MAX_ENUMERANT : Reader = InlineComposite;
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
