pub mod Node {
    use capnprust::layout::*;
    use schema_capnp::*;

    pub static STRUCT_SIZE : StructSize = StructSize {data : 3, pointers : 4,
                                                      preferredListEncoding : INLINE_COMPOSITE};

    pub struct Reader<'self> {
        _reader : StructReader<'self>
    }

    pub mod Body {
        use capnprust::layout::*;
        use schema_capnp::*;

        pub enum Reader<'self> {
            fileNode(FileNode::Reader<'self>),
            structNode(StructNode::Reader<'self>),
            enumNode(EnumNode::Reader<'self>),
            interfaceNode(InterfaceNode::Reader<'self>),
            constNode(ConstNode::Reader<'self>),
            annotationNode(AnnotationNode::Reader<'self>)
        }

        pub struct Builder {
            _builder : StructBuilder
        }

        impl Builder {
            pub fn new(builder : StructBuilder) -> Builder {
                Builder { _builder : builder }
            }

            pub fn initFileNode(&self) -> FileNode::Builder {
                self._builder.setDataField::<u16>(8, 0);
                FileNode::Builder::new(
                    self._builder.initStructField(3, FileNode::STRUCT_SIZE))
            }

            pub fn initStructNode(&self) -> StructNode::Builder {
                self._builder.setDataField::<u16>(8, 1);
                StructNode::Builder::new(
                    self._builder.initStructField(3, StructNode::STRUCT_SIZE))
            }

        }
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

        pub fn getDisplayName(&self) -> &'self str {
            self._reader.getTextField(0, "")
        }

        pub fn getScopeId(&self) -> u64 {
            self._reader.getDataField::<u64>(1)
        }

        pub fn getNestedNodes(&self) -> NestedNode::List::Reader<'self> {
            NestedNode::List::Reader::new(self._reader.getListField(1, INLINE_COMPOSITE, None))
        }

        pub fn getBody(&self) -> Body::Reader<'self> {
            match self._reader.getDataField::<u16>(8) {
                0 => {
                    return Body::fileNode(
                        FileNode::Reader::new(self._reader.getStructField(3, None)));
                }
                1 => {
                    return Body::structNode(
                        StructNode::Reader::new(self._reader.getStructField(3, None)));
                }
                2 => {
                    return Body::enumNode(
                        EnumNode::Reader::new(self._reader.getStructField(3, None)));
                }
                3 => {
                    return Body::interfaceNode(
                        InterfaceNode::Reader::new(self._reader.getStructField(3, None)));
                }
                4 => {
                    return Body::constNode(
                        ConstNode::Reader::new(self._reader.getStructField(3, None)));
                }
                5 => {
                    return Body::annotationNode(
                        AnnotationNode::Reader::new(self._reader.getStructField(3, None)));
                }
                _ => fail!("impossible")
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

    pub struct Builder {
        _builder : StructBuilder
    }

    impl Builder {
        pub fn new(builder : StructBuilder) -> Builder {
            Builder { _builder : builder }
        }
    }

    list_submodule!(schema_capnp, Node)
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
            textValue(&'self [u8]),
            dataValue, // TODO
            listValue, // TODO
            enumValue(u16),
            structValue, // TODO
            interfaceType, // TODO
            objectType // TODO
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

    list_submodule!(schema_capnp, Annotation)
}


pub mod FileNode {
    use capnprust::layout::*;

    pub static STRUCT_SIZE : StructSize = StructSize {data : 0, pointers : 1,
                                                      preferredListEncoding : POINTER};

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

        pub fn getImports(&self) -> Import::List::Reader<'self> {
            Import::List::Reader::new(self._reader.getListField(0, INLINE_COMPOSITE, None))
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

    list_submodule!(schema_capnp, FileNode)

    pub mod Import {
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

            pub fn getId(&self) -> u64 {
                self._reader.getDataField::<u64>(0)
            }

            pub fn getName(&self) -> &'self str {
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

        list_submodule!(schema_capnp, FileNode::Import)
    }
}

pub mod ElementSize {
    pub enum ElementSize {
        empty = 0,
        bit = 1,
        byte = 2,
        twoBytes = 3,
        fourBytes = 4,
        eightBytes = 5,
        pointer = 6,
        inlineComposite = 7
    }
}

pub mod StructNode {
    use capnprust::layout::*;
    use schema_capnp::*;
    use std;

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

        pub fn getDataSectionWordSize(&self) -> u16 {
            self._reader.getDataField::<u16>(0)
        }

        pub fn getPointerSectionSize(&self) -> u16 {
            self._reader.getDataField::<u16>(1)
        }

        pub fn getPreferredListEncoding(&self) -> ElementSize::ElementSize {
            unsafe {
                std::cast::transmute(self._reader.getDataField::<u16>(2) as u64)
            }
        }

        pub fn getMembers(&self) -> Member::List::Reader<'self> {
            Member::List::Reader::new(self._reader.getListField(0, INLINE_COMPOSITE, None))
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

    list_submodule!(schema_capnp, StructNode)

    pub mod Member {
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

            pub fn getName(&self) -> & 'self str {
                self._reader.getTextField(0, "")
            }

            pub fn getOrdinal(&self) -> u16 {
                self._reader.getDataField::<u16>(0)
            }

            pub fn getCodeOrder(&self) -> u16 {
                self._reader.getDataField::<u16>(1)
            }

            pub fn getAnnotations(&self) -> Annotation::List::Reader<'self> {
                Annotation::List::Reader::new(self._reader.getListField(1, INLINE_COMPOSITE, None))
            }

            pub fn getBody(&self) -> Body::Reader<'self> {
                match self._reader.getDataField::<u16>(2) {
                    0 => {
                        return Body::fieldMember(
                            StructNode::Field::Reader::new(self._reader.getStructField(2, None)));
                    }
                    1 => {
                        return Body::unionMember(
                            StructNode::Union::Reader::new(self._reader.getStructField(2, None)));
                    }
                    _ => fail!("unrecognized discriminant for StructNode::Body")
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


        list_submodule!(schema_capnp, StructNode::Member)

        pub mod Body {
            use schema_capnp::*;
            pub enum Reader<'self> {
                fieldMember(StructNode::Field::Reader<'self>),
                unionMember(StructNode::Union::Reader<'self>)
            }
        }

    }

    pub mod Field {
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

            pub fn getOffset(&self) -> u32 {
                self._reader.getDataField::<u32>(0)
            }

            pub fn getType(&self) -> Type::Reader<'self> {
                Type::Reader::new(self._reader.getStructField(0, None))
            }

            pub fn getDefaultValue(&self) -> Value::Reader<'self> {
                Value::Reader::new(self._reader.getStructField(1, None))
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

        list_submodule!(schema_capnp, StructNode::Field)
    }

    pub mod Union {
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

            pub fn getDiscriminantOffset(&self) -> u32 {
                self._reader.getDataField::<u32>(0)
            }

            pub fn getMembers(&self) -> StructNode::Member::List::Reader<'self> {
                StructNode::Member::List::Reader::new(
                    self._reader.getListField(0, INLINE_COMPOSITE, None))
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

        list_submodule!(schema_capnp, StructNode::Union)
    }


}

pub mod EnumNode {
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

    list_submodule!(schema_capnp, EnumNode)
}

pub mod InterfaceNode {
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


    list_submodule!(schema_capnp, InterfaceNode)

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

        list_submodule!(schema_capnp, InterfaceNode::Method)

    }
}

pub mod ConstNode {
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
            Type::Reader::new(self._reader.getStructField(0, None))
        }

        pub fn getValue(&self) -> Value::Reader<'self>{
            Value::Reader::new(self._reader.getStructField(1, None))
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

    list_submodule!(schema_capnp, ConstNode)

}

pub mod AnnotationNode {
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
            Type::Reader::new(self._reader.getStructField(0, None))
        }

        pub fn getTargetsFile(&self) -> bool {
            self._reader.getDataFieldBool(0)
        }

        pub fn getTargetsConst(&self) -> bool {
            self._reader.getDataFieldBool(1)
        }

        pub fn getTargetsEnum(&self) -> bool {
            self._reader.getDataFieldBool(2)
        }

        pub fn getTargetsEnumerant(&self) -> bool {
            self._reader.getDataFieldBool(3)
        }

        pub fn getTargetsStruct(&self) -> bool {
            self._reader.getDataFieldBool(4)
        }

        pub fn getTargetsField(&self) -> bool {
            self._reader.getDataFieldBool(5)
        }

        pub fn getTargetsUnion(&self) -> bool {
            self._reader.getDataFieldBool(6)
        }

        pub fn getTargetsInterface(&self) -> bool {
            self._reader.getDataFieldBool(7)
        }

        pub fn getTargetsMethod(&self) -> bool {
            self._reader.getDataFieldBool(8)
        }

        pub fn getTargetsParam(&self) -> bool {
            self._reader.getDataFieldBool(9)
        }

        pub fn getTargetsAnnotation(&self) -> bool {
            self._reader.getDataFieldBool(10)
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


    list_submodule!(schema_capnp, AnnotationNode)
}



pub mod CodeGeneratorRequest {
    use capnprust::layout::*;
    use list::*;
    use schema_capnp::*;

    pub static STRUCT_SIZE : StructSize = StructSize {data : 0, pointers : 2,
                                                      preferredListEncoding : INLINE_COMPOSITE};

    pub struct Reader<'self> {
        _reader : StructReader<'self>
    }

    impl <'self> Reader<'self> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ _reader : reader }
        }

        pub fn getRequestedFiles(&self) -> PrimitiveList::Reader<'self> {
            PrimitiveList::Reader::new(self._reader.getListField(1, EIGHT_BYTES, None))
        }

        pub fn getNodes(&self) -> Node::List::Reader<'self> {
            Node::List::Reader::new(self._reader.getListField(0, INLINE_COMPOSITE, None))
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


    list_submodule!(schema_capnp, CodeGeneratorRequest)

}
