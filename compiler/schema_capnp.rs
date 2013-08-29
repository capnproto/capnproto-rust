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
                    return Some(file_(()));
                }
                1 => {
                    return Some(struct_(
                        Struct::Reader::new(self._reader)));
                }
                2 => {
                    return Some(enum_(
                        Enum::Reader::new(self._reader)));
                }
                3 => {
                    return Some(interface(
                        Interface::Reader::new(self._reader)));
                }
                4 => {
                    return Some(const_(
                        Const::Reader::new(self._reader)));
                }
                5 => {
                    return Some(annotation(
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
        file_(()),
        struct_(Struct::Reader<'self>),
        enum_(Enum::Reader<'self>),
        interface(Interface::Reader<'self>),
        const_(Const::Reader<'self>),
        annotation(Annotation::Reader<'self>)
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
                    Some(slot(Slot::Reader::new(self._reader)))
                }
                1 => Some(group(Group::Reader::new(self._reader))),
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
        slot(Field::Slot::Reader<'self>),
        group(Field::Group::Reader<'self>)
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
                    0 => return Some(implicit(())),
                    1 => return Some(explicit(self._reader.getDataField::<u16>(6))),
                    _ => return None
                }
            }
        }

        pub enum Which {
            implicit(()),
            explicit(u16),
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
                0 => Some(void),
                1 => Some(bool_),
                2 => Some(int8),
                3 => Some(int16),
                4 => Some(int32),
                5 => Some(int64),
                6 => Some(uint8),
                7 => Some(uint16),
                8 => Some(uint32),
                9 => Some(uint64),
                10 => Some(float32),
                11 => Some(float64),
                12 => Some(text),
                13 => Some(data),
                14 => {
                    return Some(list(List_::Reader::new(self._reader)));
                }
                15 => {
                    return Some(enum_(Enum::Reader::new(self._reader)));
                }
                16 => {
                    return Some(struct_(Struct::Reader::new(self._reader)));
                }
                17 => {
                    return Some(interface(Interface::Reader::new(self._reader)));
                }
                18 => { return Some(object); }
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
        void,
        bool_,
        int8,
        int16,
        int32,
        int64,
        uint8,
        uint16,
        uint32,
        uint64,
        float32,
        float64,
        text,
        data,
        list(List_::Reader<'self>),
        enum_(Enum::Reader<'self>),
        struct_(Struct::Reader<'self>),
        interface(Interface::Reader<'self>),
        object
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
        void,
        bool_(bool),
        int8(i8),
        int16(i16),
        int32(i32),
        int64(i64),
        uint8(u8),
        uint16(u16),
        uint32(u32),
        uint64(u64),
        float32(f32),
        float64(f32),
        text(Text::Reader<'self>),
        data(Data::Reader<'self>),
        list, // TODO
        enum_(u16),
        struct_, // TODO
        interface,
        object // TODO
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
