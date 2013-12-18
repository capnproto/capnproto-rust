/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

pub mod Node {
    use std;
    use capnp::layout::{StructReader, StructBuilder, FromStructReader,
                        FromStructBuilder, StructSize, INLINE_COMPOSITE};
    use capnp::blob::Text;
    use capnp::list::{StructList};

    pub static STRUCT_SIZE : StructSize = StructSize {data : 5, pointers : 5,
                                                      preferred_list_encoding : INLINE_COMPOSITE};

    pub struct Reader<'a> {
        priv reader : StructReader<'a>
    }

    impl <'a> FromStructReader<'a> for Reader<'a> {
        fn from_struct_reader(reader: StructReader<'a>) -> Reader<'a> {
            Reader {reader : reader}
        }
    }

    impl <'a> Reader<'a> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ reader : reader }
        }

        pub fn total_size_in_words(&self) -> uint {
            self.reader.total_size() as uint
        }

        pub fn get_id(&self) -> u64 {
            self.reader.get_data_field::<u64>(0)
        }

        pub fn get_display_name(&self) -> Text::Reader<'a> {
            self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
        }

        pub fn get_display_name_prefix_length(&self) -> u32 {
            self.reader.get_data_field::<u32>(2)
        }

        pub fn get_scope_id(&self) -> u64 {
            self.reader.get_data_field::<u64>(2)
        }

        pub fn get_nested_nodes(&self) -> StructList::Reader<'a, NestedNode::Reader> {
            StructList::Reader::new(self.reader.get_pointer_field(1).get_list(INLINE_COMPOSITE, std::ptr::null()))
        }

        pub fn which(&self) -> Option<Which<'a>> {
            match self.reader.get_data_field::<u16>(6) {
                0 => {
                    return Some(File(()));
                }
                1 => {
                    return Some(Struct(
                        Struct::Reader::new(self.reader)));
                }
                2 => {
                    return Some(Enum(
                        Enum::Reader::new(self.reader)));
                }
                3 => {
                    return Some(Interface(
                        Interface::Reader::new(self.reader)));
                }
                4 => {
                    return Some(Const(
                        Const::Reader::new(self.reader)));
                }
                5 => {
                    return Some(Annotation(
                        Annotation::Reader::new(self.reader)));
                }
                _ => return None
            }
        }
    }

    pub struct Builder<'a> {
        priv builder : StructBuilder<'a>
    }

    impl <'a> FromStructBuilder<'a> for Builder<'a> {
        fn from_struct_builder(builder: StructBuilder<'a>) -> Builder<'a> {
            Builder {builder : builder}
        }
    }


    impl <'a> Builder<'a> {
        pub fn new(builder : StructBuilder<'a>) -> Builder<'a> {
            Builder { builder : builder }
        }

/*
            pub fn initFileNode(&self) -> Node::File::Builder {
                self.builder.setDataField::<u16>(8, 0);
                FileNode::Builder::new(
                    self.builder.initStructField(3, FileNode::STRUCT_SIZE))
            }

            pub fn initStructNode(&self) -> Node::Struct::Builder {
                self.builder.setDataField::<u16>(8, 1);
                StructNode::Builder::new(
                    self.builder.initStructField(3, StructNode::STRUCT_SIZE))
            }
*/

    }

    pub enum Which<'a> {
        File(()),
        Struct(Struct::Reader<'a>),
        Enum(Enum::Reader<'a>),
        Interface(Interface::Reader<'a>),
        Const(Const::Reader<'a>),
        Annotation(Annotation::Reader<'a>)
    }

    pub mod Struct {
        use std;
        use capnp::layout;
        use capnp::list::{StructList};
        use schema_capnp;

        pub struct Reader<'a> {
            priv reader : layout::StructReader<'a>
        }

        impl <'a> Reader<'a> {

            pub fn new<'a>(reader : layout::StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn total_size_in_words(&self) -> uint {
                self.reader.total_size() as uint
            }

            pub fn get_data_word_count(&self) -> u16 {
                self.reader.get_data_field::<u16>(7)
            }

            pub fn get_pointer_count(&self) -> u16 {
                self.reader.get_data_field::<u16>(12)
            }

            pub fn get_preferred_list_encoding(&self) ->
                Option<schema_capnp::ElementSize::Reader> {
                FromPrimitive::from_u16(self.reader.get_data_field::<u16>(13))
            }

            pub fn get_is_group(&self) -> bool {
                self.reader.get_bool_field(224)
            }

            pub fn get_discriminant_count(&self) -> u16 {
                self.reader.get_data_field::<u16>(15)
            }

            pub fn get_discriminant_offset(&self) -> u32 {
                self.reader.get_data_field::<u32>(8)
            }

            pub fn get_fields(&self) -> StructList::Reader<'a, schema_capnp::Field::Reader> {
                StructList::Reader::new(
                    self.reader.get_pointer_field(3).get_list(layout::INLINE_COMPOSITE, std::ptr::null()))
            }
        }

        pub struct Builder<'a> {
            priv builder : layout::StructBuilder<'a>
        }

        impl <'a>Builder<'a> {
            pub fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
                Builder { builder : builder }
            }
        }
    }

    pub mod Enum {
        use std;
        use schema_capnp;
        use capnp::layout;
        use capnp::list::StructList;

        pub struct Reader<'a> {
            priv reader : layout::StructReader<'a>
        }

        impl <'a> Reader<'a> {

            pub fn new<'a>(reader : layout::StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn total_size_in_words(&self) -> uint {
                self.reader.total_size() as uint
            }

            pub fn get_enumerants(&self) -> StructList::Reader<'a, schema_capnp::Enumerant::Reader> {
                StructList::Reader::new(
                      self.reader.get_pointer_field(3).get_list(
                        schema_capnp::Enumerant::STRUCT_SIZE.preferred_list_encoding,
                        std::ptr::null()))
            }

        }

        pub struct Builder<'a> {
            priv builder : layout::StructBuilder<'a>
        }

        impl <'a> Builder<'a> {
            pub fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
                Builder { builder : builder }
            }
        }

    }

    pub mod Interface {
        use capnp::layout;

        pub struct Reader<'a> {
            priv reader : layout::StructReader<'a>
        }

        impl <'a> Reader<'a> {

            pub fn new<'a>(reader : layout::StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn total_size_in_words(&self) -> uint {
                self.reader.total_size() as uint
            }

            // TODO methods
        }

        pub struct Builder<'a> {
            priv builder : layout::StructBuilder<'a>
        }

        impl <'a> Builder<'a> {
            pub fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
                Builder { builder : builder }
            }
        }

    }

    pub mod Const {
        use std;
        use capnp::layout;
        use schema_capnp;

        pub struct Reader<'a> {
            priv reader : layout::StructReader<'a>
        }

        impl <'a> Reader<'a> {

            pub fn new<'a>(reader : layout::StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn total_size_in_words(&self) -> uint {
                self.reader.total_size() as uint
            }

            pub fn get_type(&self) -> schema_capnp::Type::Reader<'a> {
                schema_capnp::Type::Reader::new(self.reader.get_pointer_field(3).get_struct(std::ptr::null()))
            }

            pub fn get_value(&self) -> schema_capnp::Value::Reader<'a>{
                schema_capnp::Value::Reader::new(self.reader.get_pointer_field(4).get_struct(std::ptr::null()))
            }
        }

        pub struct Builder<'a> {
            priv builder : layout::StructBuilder<'a>
        }

        impl <'a> Builder<'a> {
            pub fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
                Builder { builder : builder }
            }
        }
    }

    pub mod Annotation {
        use std;
        use capnp::layout::*;
        use schema_capnp::*;

        pub struct Reader<'a> {
            priv reader : StructReader<'a>
        }

        impl <'a> Reader<'a> {

            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn total_size_in_words(&self) -> uint {
                self.reader.total_size() as uint
            }

            pub fn get_type(&self) -> Type::Reader<'a> {
                Type::Reader::new(self.reader.get_pointer_field(3).get_struct(std::ptr::null()))
            }

            pub fn get_targets_file(&self) -> bool {
                self.reader.get_bool_field(112)
            }

            pub fn get_targets_const(&self) -> bool {
                self.reader.get_bool_field(113)
            }

            pub fn get_targets_enum(&self) -> bool {
                self.reader.get_bool_field(114)
            }

            pub fn get_targets_enumerant(&self) -> bool {
                self.reader.get_bool_field(115)
            }

            pub fn get_targets_struct(&self) -> bool {
                self.reader.get_bool_field(116)
            }

            pub fn get_targets_field(&self) -> bool {
                self.reader.get_bool_field(117)
            }

            pub fn get_targets_union(&self) -> bool {
                self.reader.get_bool_field(118)
            }

            pub fn get_targets_group(&self) -> bool {
                self.reader.get_bool_field(119)
            }

            pub fn get_targets_interface(&self) -> bool {
                self.reader.get_bool_field(120)
            }

            pub fn get_targets_method(&self) -> bool {
                self.reader.get_bool_field(121)
            }

            pub fn get_targets_param(&self) -> bool {
                self.reader.get_bool_field(122)
            }

            pub fn get_targets_annotation(&self) -> bool {
                self.reader.get_bool_field(123)
            }

        }
        pub struct Builder<'a> {
            priv builder : StructBuilder<'a>
        }

        impl <'a>Builder<'a> {
            pub fn new(builder : StructBuilder<'a>) -> Builder<'a> {
                Builder { builder : builder }
            }
        }
   }

    pub mod NestedNode {
        use std;
        use capnp::layout::*;
        pub struct Reader<'a> {
            priv reader : StructReader<'a>
        }

        impl <'a> FromStructReader<'a> for Reader<'a> {
            fn from_struct_reader(reader: StructReader<'a>) -> Reader<'a> {
                Reader {reader : reader}
            }
        }

        impl <'a> Reader<'a> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn get_name(&self) -> &'a str {
                self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
            }

            pub fn get_id(&self) -> u64 {
                self.reader.get_data_field::<u64>(0)
            }
        }

        pub struct Builder<'a> {
            priv builder : StructBuilder<'a>
        }

        impl <'a>Builder<'a> {
            pub fn new(builder : StructBuilder<'a>) -> Builder<'a> {
                Builder { builder : builder }
            }
        }
    }

}

pub mod Field {
    use std;
    use capnp::layout::*;
    use capnp::blob::*;
    use schema_capnp::*;

    pub static STRUCT_SIZE : StructSize =
        StructSize {data : 3, pointers : 4,
        preferred_list_encoding : INLINE_COMPOSITE};

    pub static NO_DISCRIMINANT : u16 = 0xffffu16;

    pub struct Reader<'a> {
        priv reader : StructReader<'a>
    }

    impl <'a> FromStructReader<'a> for Reader<'a> {
        fn from_struct_reader(reader: StructReader<'a>) -> Reader<'a> {
            Reader {reader : reader}
        }
    }

    impl <'a> Reader<'a> {
        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ reader : reader }
        }

        pub fn get_name(&self) -> Text::Reader<'a> {
            self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
        }

        pub fn get_code_order(&self) -> u16 {
            self.reader.get_data_field::<u16>(0)
        }

        pub fn get_discriminant_value(&self) -> u16 {
            self.reader.get_data_field_mask::<u16>(1, 0xffff)
        }

        pub fn which(&self) -> Option<Which<'a>> {
            match self.reader.get_data_field::<u16>(4) {
                0 => {
                    Some(Slot(Slot::Reader::new(self.reader)))
                }
                1 => Some(Group(Group::Reader::new(self.reader))),
                _ => None
            }
        }

        pub fn get_ordinal(&self) -> Ordinal::Reader<'a> {
            Ordinal::Reader::new(self.reader)
        }
    }

    pub struct Builder<'a> {
        priv builder : StructBuilder<'a>
    }

    impl <'a>Builder<'a> {
        pub fn new(builder : StructBuilder<'a>) -> Builder<'a> {
            Builder { builder : builder }
        }
    }

    pub enum Which<'a> {
        Slot(Field::Slot::Reader<'a>),
        Group(Field::Group::Reader<'a>)
    }

    pub mod Slot {
        use std;
        use capnp::layout::*;
        use schema_capnp::*;

        pub struct Reader<'a> {
            priv reader : StructReader<'a>
        }

        impl <'a> Reader<'a> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn get_offset(&self) -> u32 {
                self.reader.get_data_field::<u32>(1)
            }

            pub fn get_type(&self) -> Type::Reader<'a> {
                Type::Reader::new(self.reader.get_pointer_field(2).get_struct(std::ptr::null()))
            }

            pub fn get_default_value(&self) -> Value::Reader<'a> {
                Value::Reader::new(self.reader.get_pointer_field(3).get_struct(std::ptr::null()))
            }
        }
    }

    pub mod Group {
        use capnp::layout::*;

        pub struct Reader<'a> {
            priv reader : StructReader<'a>
        }

        impl <'a> Reader<'a> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn get_type_id(&self) -> u64 {
                self.reader.get_data_field::<u64>(2)
            }
        }
    }


    pub mod Ordinal {
        use capnp::layout::*;

        pub struct Reader<'a> {
            priv reader : StructReader<'a>
        }

        impl <'a> Reader<'a> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn which(&self) -> Option<Which> {
                match self.reader.get_data_field::<u16>(4) {
                    0 => return Some(Implicit(())),
                    1 => return Some(Explicit(self.reader.get_data_field::<u16>(6))),
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
    use std;
    use capnp::layout::*;
    use capnp::list::StructList;
    use schema_capnp::*;

    pub static STRUCT_SIZE : StructSize =
        StructSize {data : 1, pointers : 2,
        preferred_list_encoding : INLINE_COMPOSITE};

    pub struct Reader<'a> {
        priv reader : StructReader<'a>
    }

    impl <'a> FromStructReader<'a> for Reader<'a> {
        fn from_struct_reader(reader: StructReader<'a>) -> Reader<'a> {
            Reader {reader : reader}
        }
    }

    impl <'a> Reader<'a> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ reader : reader }
        }

        pub fn total_size_in_words(&self) -> uint {
            self.reader.total_size() as uint
        }

        pub fn get_name(&self) -> &'a str {
            self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
        }

        pub fn get_code_order(&self) -> u16 {
            self.reader.get_data_field::<u16>(0)
        }

        pub fn get_annotations(&self) -> StructList::Reader<'a, Annotation::Reader> {
            StructList::Reader::new(
                self.reader.get_pointer_field(1).get_list(
                    Annotation::STRUCT_SIZE.preferred_list_encoding,
                    std::ptr::null()))
        }
    }

    pub struct Builder<'a> {
        priv builder : StructBuilder<'a>
    }

    impl <'a>Builder<'a> {
        pub fn new(builder : StructBuilder<'a>) -> Builder<'a> {
            Builder { builder : builder }
        }
    }

}

pub mod Method {
    use capnp::layout::*;

    pub struct Reader<'a> {
        priv reader : StructReader<'a>
    }

    impl <'a> Reader<'a> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ reader : reader }
        }

        pub fn total_size_in_words(&self) -> uint {
            self.reader.total_size() as uint
        }
    }


    pub struct Builder<'a> {
        priv builder : StructBuilder<'a>
    }

    impl <'a>Builder<'a> {
        pub fn new(builder : StructBuilder<'a>) -> Builder<'a> {
            Builder { builder : builder }
        }
    }
}


pub mod Type {
    use capnp::layout::*;

    pub struct Reader<'a> {
        priv reader : StructReader<'a>
    }

    impl <'a> Reader<'a> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ reader : reader }
        }

        pub fn total_size_in_words(&self) -> uint {
            self.reader.total_size() as uint
        }

        pub fn which(&self) -> Option<Which<'a>> {
            match self.reader.get_data_field::<u16>(0) {
                0 => Some(Void(())),
                1 => Some(Bool(())),
                2 => Some(Int8(())),
                3 => Some(Int16(())),
                4 => Some(Int32(())),
                5 => Some(Int64(())),
                6 => Some(Uint8(())),
                7 => Some(Uint16(())),
                8 => Some(Uint32(())),
                9 => Some(Uint64(())),
                10 => Some(Float32(())),
                11 => Some(Float64(())),
                12 => Some(Text(())),
                13 => Some(Data(())),
                14 => {
                    return Some(List(List::Reader::new(self.reader)));
                }
                15 => {
                    return Some(Enum(Enum::Reader::new(self.reader)));
                }
                16 => {
                    return Some(Struct(Struct::Reader::new(self.reader)));
                }
                17 => {
                    return Some(Interface(Interface::Reader::new(self.reader)));
                }
                18 => { return Some(AnyPointer(())); }
                _ => { return None; }
            }
        }
    }

    pub struct Builder<'a> {
        priv builder : StructBuilder<'a>
    }

    impl <'a>Builder<'a> {
        pub fn new(builder : StructBuilder<'a>) -> Builder<'a> {
            Builder { builder : builder }
        }
    }

    pub enum Which<'a> {
        Void(()),
        Bool(()),
        Int8(()),
        Int16(()),
        Int32(()),
        Int64(()),
        Uint8(()),
        Uint16(()),
        Uint32(()),
        Uint64(()),
        Float32(()),
        Float64(()),
        Text(()),
        Data(()),
        List(List::Reader<'a>),
        Enum(Enum::Reader<'a>),
        Struct(Struct::Reader<'a>),
        Interface(Interface::Reader<'a>),
        AnyPointer(())
    }

    pub mod List {
        use std;
        use capnp::layout::*;
        use schema_capnp::*;


        pub struct Reader<'a> {
            priv reader : StructReader<'a>
        }

        impl <'a> Reader<'a> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn get_element_type(&self) -> Type::Reader<'a> {
                Type::Reader::new(self.reader.get_pointer_field(0).get_struct(std::ptr::null()))
            }
        }
    }

    pub mod Enum {
        use capnp::layout::*;

        pub struct Reader<'a> {
            priv reader : StructReader<'a>
        }

        impl <'a> Reader<'a> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn get_type_id(&self) -> u64 {
                self.reader.get_data_field::<u64>(1)
            }
        }
    }

    pub mod Struct {
        use capnp::layout::*;

        pub struct Reader<'a> {
            priv reader : StructReader<'a>
        }

        impl <'a> Reader<'a> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn get_type_id(&self) -> u64 {
                self.reader.get_data_field::<u64>(1)
            }
        }
    }

    pub mod Interface {
        use capnp::layout::*;

        pub struct Reader<'a> {
            priv reader : StructReader<'a>
        }

        impl <'a> Reader<'a> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn get_type_id(&self) -> u64 {
                self.reader.get_data_field::<u64>(1)
            }
        }
    }
}

pub mod Value {
    use std;
    use capnp::layout::*;
    use capnp::blob::*;
    use capnp::any;

    pub struct Reader<'a> {
        priv reader : StructReader<'a>
    }

    impl <'a> Reader<'a> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ reader : reader }
        }

        pub fn total_size_in_words(&self) -> uint {
            self.reader.total_size() as uint
        }

        pub fn which(&self) -> Option<Which<'a>> {
            match self.reader.get_data_field::<u16>(0) {
                0 => Some(Void(())),
                1 => Some(Bool(self.reader.get_bool_field(16))),
                2 => Some(Int8(self.reader.get_data_field::<i8>(2))),
                3 => Some(Int16(self.reader.get_data_field::<i16>(1))),
                4 => Some(Int32(self.reader.get_data_field::<i32>(1))),
                5 => Some(Int64(self.reader.get_data_field::<i64>(1))),
                6 => Some(Uint8(self.reader.get_data_field::<u8>(2))),
                7 => Some(Uint16(self.reader.get_data_field::<u16>(1))),
                8 => Some(Uint32(self.reader.get_data_field::<u32>(1))),
                9 => Some(Uint64(self.reader.get_data_field::<u64>(1))),
                10 => Some(Float32(self.reader.get_data_field::<f32>(1))),
                11 => Some(Float64(self.reader.get_data_field::<f64>(1))),
                12 => Some(Text(self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0))),
                13 => Some(Data(self.reader.get_pointer_field(0).get_data(std::ptr::null(), 0))),
                14 => {
                    Some(List(any::AnyPointer::Reader::new(self.reader.get_pointer_field(0))))
                }
                15 => {
                    Some(Enum(self.reader.get_data_field::<u16>(1)))
                }
                16 => {
                  Some(Struct(any::AnyPointer::Reader::new(self.reader.get_pointer_field(0))))
                }
                17 => {
                    Some(Interface)
                }
                18 => {
                  Some(AnyPointer(any::AnyPointer::Reader::new(self.reader.get_pointer_field(0))))
                }
                _ => { return None; }
            }
        }
    }

    pub struct Builder<'a> {
        priv builder : StructBuilder<'a>
    }

    impl <'a>Builder<'a> {
        pub fn new(builder : StructBuilder<'a>) -> Builder<'a> {
            Builder { builder : builder }
        }
    }

    pub enum Which<'a> {
        Void(()),
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
        Float64(f64),
        Text(Text::Reader<'a>),
        Data(Data::Reader<'a>),
        List(any::AnyPointer::Reader<'a>),
        Enum(u16),
        Struct(any::AnyPointer::Reader<'a>),
        Interface,
        AnyPointer(any::AnyPointer::Reader<'a>)
    }
}

pub mod Annotation {
    use std;
    use capnp::layout::*;
    use schema_capnp::*;

    pub static STRUCT_SIZE : StructSize = StructSize {data : 1, pointers : 1,
                                                      preferred_list_encoding : INLINE_COMPOSITE};

    pub struct Reader<'a> {
        priv reader : StructReader<'a>
    }

    impl <'a> FromStructReader<'a> for Reader<'a> {
        fn from_struct_reader(reader: StructReader<'a>) -> Reader<'a> {
            Reader {reader : reader}
        }
    }

    impl <'a> Reader<'a> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ reader : reader }
        }

        pub fn total_size_in_words(&self) -> uint {
            self.reader.total_size() as uint
        }

        pub fn get_id(&self) -> u64 {
            self.reader.get_data_field::<u64>(0)
        }

        pub fn get_value(&self) -> Value::Reader<'a> {
            Value::Reader::new(self.reader.get_pointer_field(0).get_struct(std::ptr::null()))
        }
    }

    pub struct Builder<'a> {
        priv builder : StructBuilder<'a>
    }

    impl <'a>Builder<'a> {
        pub fn new(builder : StructBuilder<'a>) -> Builder<'a> {
            Builder { builder : builder }
        }
    }
}

pub mod ElementSize {

    #[repr(u16)]
    #[deriving(FromPrimitive)]
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
}




pub mod CodeGeneratorRequest {
    use std;
    use capnp::layout::{StructSize, StructReader, INLINE_COMPOSITE, StructBuilder, FromStructReader};
    use capnp::list::StructList;
    use schema_capnp::*;

    pub static STRUCT_SIZE : StructSize = StructSize {data : 0, pointers : 2,
                                                      preferred_list_encoding : INLINE_COMPOSITE};

    pub struct Reader<'a> {
        priv reader : StructReader<'a>
    }

    impl <'a> FromStructReader<'a> for Reader<'a> {
        fn from_struct_reader(reader: StructReader<'a>) -> Reader<'a> {
            Reader {reader : reader}
        }
    }

    impl <'a> Reader<'a> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ reader : reader }
        }

        pub fn get_nodes(&self) -> StructList::Reader<'a, Node::Reader> {
            StructList::Reader::new(self.reader.get_pointer_field(0).get_list(INLINE_COMPOSITE, std::ptr::null()))
        }

        pub fn get_requested_files(&self) -> StructList::Reader<'a, RequestedFile::Reader> {
            StructList::Reader::new(
                 self.reader.get_pointer_field(1).get_list(
                    RequestedFile::STRUCT_SIZE.preferred_list_encoding,
                    std::ptr::null()))
        }

    }

    pub struct Builder<'a> {
        priv builder : StructBuilder<'a>
    }

    impl <'a>Builder<'a> {
        pub fn new(builder : StructBuilder<'a>) -> Builder<'a> {
            Builder { builder : builder }
        }
        pub fn init_nodes(&self, size : uint) -> StructList::Builder<'a, Node::Builder<'a>> {
            StructList::Builder::new(
                self.builder.get_pointer_field(0).init_struct_list(size, Node::STRUCT_SIZE))
        }
    }

    pub mod RequestedFile {
        use std;
        use capnp::layout::*;
        use capnp::blob::*;
        use capnp::list::StructList;

        pub static STRUCT_SIZE : StructSize =
            StructSize {data : 1, pointers : 2,
            preferred_list_encoding : INLINE_COMPOSITE};

        pub struct Reader<'a> {
            priv reader : StructReader<'a>
        }

        impl <'a> FromStructReader<'a> for Reader<'a> {
            fn from_struct_reader(reader: StructReader<'a>) -> Reader<'a> {
                Reader {reader : reader}
            }
        }

        impl <'a> Reader<'a> {
            pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                Reader{ reader : reader }
            }

            pub fn get_id(&self) -> u64 {
                self.reader.get_data_field::<u64>(0)
            }

            pub fn get_filename(&self) -> Text::Reader<'a> {
                self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
            }

            pub fn get_imports(&self) -> StructList::Reader<'a, Import::Reader> {
                StructList::Reader::new(
                 self.reader.get_pointer_field(1).get_list(
                        Import::STRUCT_SIZE.preferred_list_encoding,
                        std::ptr::null()))
            }
        }

        pub struct Builder<'a> {
            priv builder : StructBuilder<'a>
        }

        impl <'a>Builder<'a> {
            pub fn new(builder : StructBuilder<'a>) -> Builder<'a> {
                Builder { builder : builder }
            }
        }

        pub mod Import {
            use std;
            use capnp::layout::*;
            use capnp::blob::*;

            pub static STRUCT_SIZE : StructSize =
                StructSize {data : 1, pointers : 1,
                preferred_list_encoding : INLINE_COMPOSITE};

            pub struct Reader<'a> {
                priv reader : StructReader<'a>
            }

            impl <'a> FromStructReader<'a> for Reader<'a> {
                fn from_struct_reader(reader: StructReader<'a>) -> Reader<'a> {
                    Reader {reader : reader}
                }
            }

            impl <'a> Reader<'a> {
                pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
                    Reader{ reader : reader }
                }

                pub fn get_id(&self) -> u64 {
                    self.reader.get_data_field::<u64>(0)
                }

                pub fn get_name(&self) -> Text::Reader<'a> {
                    self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
                }
            }

            pub struct Builder<'a> {
                priv builder : StructBuilder<'a>
            }

            impl <'a>Builder<'a> {
                pub fn new(builder : StructBuilder<'a>) -> Builder<'a> {
                    Builder { builder : builder }
                }
            }
        }
    }
}
