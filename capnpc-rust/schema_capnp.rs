#![allow(unused_imports)]
#![allow(dead_code)]

pub mod Node {
  use std;
  use capnp::any::any_pointer;
  use capnp::capability::{FromClientHook, FromTypelessPipeline};
  use capnp::blob::{text, data};
  use capnp::layout;
  use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
  use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
  use schema_capnp;

  pub static STRUCT_SIZE : layout::StructSize =
    layout::StructSize { data : 5, pointers : 5, preferred_list_encoding : layout::InlineComposite};


  pub struct Reader<'a> { reader : layout::StructReader<'a> }

  impl <'a> layout::FromStructReader<'a> for Reader<'a> {
    fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
      Reader { reader : reader }
    }
  }

  impl <'a> layout::ToStructReader<'a> for Reader<'a> {
    fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
  }

  impl <'a> Reader<'a> {
    #[inline]
    pub fn get_id(&self) -> u64 {
      self.reader.get_data_field::<u64>(0)
    }
    #[inline]
    pub fn get_display_name(&self) -> text::Reader<'a> {
      self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
    }
    pub fn has_display_name(&self) -> bool {
      !self.reader.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn get_display_name_prefix_length(&self) -> u32 {
      self.reader.get_data_field::<u32>(2)
    }
    #[inline]
    pub fn get_scope_id(&self) -> u64 {
      self.reader.get_data_field::<u64>(2)
    }
    #[inline]
    pub fn get_nested_nodes(&self) -> struct_list::Reader<'a,schema_capnp::Node::NestedNode::Reader<'a>> {
      struct_list::Reader::new(self.reader.get_pointer_field(1).get_list(schema_capnp::Node::NestedNode::STRUCT_SIZE.preferred_list_encoding, std::ptr::null()))
    }
    pub fn has_nested_nodes(&self) -> bool {
      !self.reader.get_pointer_field(1).is_null()
    }
    #[inline]
    pub fn get_annotations(&self) -> struct_list::Reader<'a,schema_capnp::Annotation::Reader<'a>> {
      struct_list::Reader::new(self.reader.get_pointer_field(2).get_list(schema_capnp::Annotation::STRUCT_SIZE.preferred_list_encoding, std::ptr::null()))
    }
    pub fn has_annotations(&self) -> bool {
      !self.reader.get_pointer_field(2).is_null()
    }
    #[inline]
    pub fn which(&self) -> std::option::Option<WhichReader<'a>> {
      match self.reader.get_data_field::<u16>(6) {
        0 => {
          return std::option::Some(File(
            ()
          ));
        }
        1 => {
          return std::option::Some(Struct(
            FromStructReader::new(self.reader)
          ));
        }
        2 => {
          return std::option::Some(Enum(
            FromStructReader::new(self.reader)
          ));
        }
        3 => {
          return std::option::Some(Interface(
            FromStructReader::new(self.reader)
          ));
        }
        4 => {
          return std::option::Some(Const(
            FromStructReader::new(self.reader)
          ));
        }
        5 => {
          return std::option::Some(Annotation(
            FromStructReader::new(self.reader)
          ));
        }
        _ => return std::option::None
      }
    }
  }

  pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
  impl <'a> layout::HasStructSize for Builder<'a> {
    #[inline]
    fn struct_size(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }
  }
  impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
    fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
      Builder { builder : builder }
    }
  }
  impl <'a> Builder<'a> {
    pub fn as_reader(&self) -> Reader<'a> {
      FromStructReader::new(self.builder.as_reader())
    }
    #[inline]
    pub fn get_id(&self) -> u64 {
      self.builder.get_data_field::<u64>(0)
    }
    #[inline]
    pub fn set_id(&self, value : u64) {
      self.builder.set_data_field::<u64>(0, value);
    }
    #[inline]
    pub fn get_display_name(&self) -> text::Builder<'a> {
      self.builder.get_pointer_field(0).get_text(std::ptr::null(), 0)
    }
    #[inline]
    pub fn set_display_name(&self, value : text::Reader) {
      self.builder.get_pointer_field(0).set_text(value);
    }
    #[inline]
    pub fn init_display_name(&self, size : uint) -> text::Builder<'a> {
      self.builder.get_pointer_field(0).init_text(size)
    }
    pub fn has_display_name(&self) -> bool {
      !self.builder.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn get_display_name_prefix_length(&self) -> u32 {
      self.builder.get_data_field::<u32>(2)
    }
    #[inline]
    pub fn set_display_name_prefix_length(&self, value : u32) {
      self.builder.set_data_field::<u32>(2, value);
    }
    #[inline]
    pub fn get_scope_id(&self) -> u64 {
      self.builder.get_data_field::<u64>(2)
    }
    #[inline]
    pub fn set_scope_id(&self, value : u64) {
      self.builder.set_data_field::<u64>(2, value);
    }
    #[inline]
    pub fn get_nested_nodes(&self) -> struct_list::Builder<'a,schema_capnp::Node::NestedNode::Builder<'a>> {
      struct_list::Builder::new(self.builder.get_pointer_field(1).get_struct_list(schema_capnp::Node::NestedNode::STRUCT_SIZE, std::ptr::null()))
    }
    #[inline]
    pub fn set_nested_nodes(&self, value : struct_list::Reader<'a,schema_capnp::Node::NestedNode::Reader<'a>>) {
      self.builder.get_pointer_field(1).set_list(&value.reader)
    }
    #[inline]
    pub fn init_nested_nodes(&self, size : uint) -> struct_list::Builder<'a,schema_capnp::Node::NestedNode::Builder<'a>> {
      struct_list::Builder::<'a, schema_capnp::Node::NestedNode::Builder<'a>>::new(
        self.builder.get_pointer_field(1).init_struct_list(size, schema_capnp::Node::NestedNode::STRUCT_SIZE))
    }
    pub fn has_nested_nodes(&self) -> bool {
      !self.builder.get_pointer_field(1).is_null()
    }
    #[inline]
    pub fn get_annotations(&self) -> struct_list::Builder<'a,schema_capnp::Annotation::Builder<'a>> {
      struct_list::Builder::new(self.builder.get_pointer_field(2).get_struct_list(schema_capnp::Annotation::STRUCT_SIZE, std::ptr::null()))
    }
    #[inline]
    pub fn set_annotations(&self, value : struct_list::Reader<'a,schema_capnp::Annotation::Reader<'a>>) {
      self.builder.get_pointer_field(2).set_list(&value.reader)
    }
    #[inline]
    pub fn init_annotations(&self, size : uint) -> struct_list::Builder<'a,schema_capnp::Annotation::Builder<'a>> {
      struct_list::Builder::<'a, schema_capnp::Annotation::Builder<'a>>::new(
        self.builder.get_pointer_field(2).init_struct_list(size, schema_capnp::Annotation::STRUCT_SIZE))
    }
    pub fn has_annotations(&self) -> bool {
      !self.builder.get_pointer_field(2).is_null()
    }
    #[inline]
    pub fn set_file(&self, _value : ()) {
      self.builder.set_data_field::<u16>(6, 0);
    }
    #[inline]
    pub fn init_struct(&self, ) -> schema_capnp::Node::Struct::Builder<'a> {
      self.builder.set_data_field::<u16>(6, 1);
      self.builder.set_data_field::<u16>(7, 0);
      self.builder.set_data_field::<u16>(12, 0);
      self.builder.set_data_field::<u16>(13, 0);
      self.builder.set_bool_field(224, false);
      self.builder.set_data_field::<u16>(15, 0);
      self.builder.set_data_field::<u32>(8, 0);
      self.builder.get_pointer_field(3).clear();
      FromStructBuilder::new(self.builder)
    }
    #[inline]
    pub fn init_enum(&self, ) -> schema_capnp::Node::Enum::Builder<'a> {
      self.builder.set_data_field::<u16>(6, 2);
      self.builder.get_pointer_field(3).clear();
      FromStructBuilder::new(self.builder)
    }
    #[inline]
    pub fn init_interface(&self, ) -> schema_capnp::Node::Interface::Builder<'a> {
      self.builder.set_data_field::<u16>(6, 3);
      self.builder.get_pointer_field(3).clear();
      self.builder.get_pointer_field(4).clear();
      FromStructBuilder::new(self.builder)
    }
    #[inline]
    pub fn init_const(&self, ) -> schema_capnp::Node::Const::Builder<'a> {
      self.builder.set_data_field::<u16>(6, 4);
      self.builder.get_pointer_field(3).clear();
      self.builder.get_pointer_field(4).clear();
      FromStructBuilder::new(self.builder)
    }
    #[inline]
    pub fn init_annotation(&self, ) -> schema_capnp::Node::Annotation::Builder<'a> {
      self.builder.set_data_field::<u16>(6, 5);
      self.builder.get_pointer_field(3).clear();
      self.builder.set_bool_field(112, false);
      self.builder.set_bool_field(113, false);
      self.builder.set_bool_field(114, false);
      self.builder.set_bool_field(115, false);
      self.builder.set_bool_field(116, false);
      self.builder.set_bool_field(117, false);
      self.builder.set_bool_field(118, false);
      self.builder.set_bool_field(119, false);
      self.builder.set_bool_field(120, false);
      self.builder.set_bool_field(121, false);
      self.builder.set_bool_field(122, false);
      self.builder.set_bool_field(123, false);
      FromStructBuilder::new(self.builder)
    }
    #[inline]
    pub fn which(&self) -> std::option::Option<WhichBuilder<'a>> {
      match self.builder.get_data_field::<u16>(6) {
        0 => {
          return std::option::Some(File(
            ()
          ));
        }
        1 => {
          return std::option::Some(Struct(
            FromStructBuilder::new(self.builder)
          ));
        }
        2 => {
          return std::option::Some(Enum(
            FromStructBuilder::new(self.builder)
          ));
        }
        3 => {
          return std::option::Some(Interface(
            FromStructBuilder::new(self.builder)
          ));
        }
        4 => {
          return std::option::Some(Const(
            FromStructBuilder::new(self.builder)
          ));
        }
        5 => {
          return std::option::Some(Annotation(
            FromStructBuilder::new(self.builder)
          ));
        }
        _ => return std::option::None
      }
    }
  }

  pub struct Pipeline { _typeless : any_pointer::Pipeline }
  impl FromTypelessPipeline for Pipeline {
    fn new(typeless : any_pointer::Pipeline) -> Pipeline {
      Pipeline { _typeless : typeless }
    }
  }
  impl Pipeline {
  }
  pub enum Which<'a,A0,A1,A2,A3,A4> {
    File(()),
    Struct(A0),
    Enum(A1),
    Interface(A2),
    Const(A3),
    Annotation(A4),
  }
  pub type WhichReader<'a> = Which<'a,schema_capnp::Node::Struct::Reader<'a>,schema_capnp::Node::Enum::Reader<'a>,schema_capnp::Node::Interface::Reader<'a>,schema_capnp::Node::Const::Reader<'a>,schema_capnp::Node::Annotation::Reader<'a>>;
  pub type WhichBuilder<'a> = Which<'a,schema_capnp::Node::Struct::Builder<'a>,schema_capnp::Node::Enum::Builder<'a>,schema_capnp::Node::Interface::Builder<'a>,schema_capnp::Node::Const::Builder<'a>,schema_capnp::Node::Annotation::Builder<'a>>;

  pub mod NestedNode {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub static STRUCT_SIZE : layout::StructSize =
      layout::StructSize { data : 1, pointers : 1, preferred_list_encoding : layout::InlineComposite};


    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn get_name(&self) -> text::Reader<'a> {
        self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
      }
      pub fn has_name(&self) -> bool {
        !self.reader.get_pointer_field(0).is_null()
      }
      #[inline]
      pub fn get_id(&self) -> u64 {
        self.reader.get_data_field::<u64>(0)
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::HasStructSize for Builder<'a> {
      #[inline]
      fn struct_size(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }
    }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn get_name(&self) -> text::Builder<'a> {
        self.builder.get_pointer_field(0).get_text(std::ptr::null(), 0)
      }
      #[inline]
      pub fn set_name(&self, value : text::Reader) {
        self.builder.get_pointer_field(0).set_text(value);
      }
      #[inline]
      pub fn init_name(&self, size : uint) -> text::Builder<'a> {
        self.builder.get_pointer_field(0).init_text(size)
      }
      pub fn has_name(&self) -> bool {
        !self.builder.get_pointer_field(0).is_null()
      }
      #[inline]
      pub fn get_id(&self) -> u64 {
        self.builder.get_data_field::<u64>(0)
      }
      #[inline]
      pub fn set_id(&self, value : u64) {
        self.builder.set_data_field::<u64>(0, value);
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
    }
  }

  pub mod Struct {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn get_data_word_count(&self) -> u16 {
        self.reader.get_data_field::<u16>(7)
      }
      #[inline]
      pub fn get_pointer_count(&self) -> u16 {
        self.reader.get_data_field::<u16>(12)
      }
      #[inline]
      pub fn get_preferred_list_encoding(&self) -> Option<schema_capnp::ElementSize::Reader> {
        FromPrimitive::from_u16(self.reader.get_data_field::<u16>(13))
      }
      #[inline]
      pub fn get_is_group(&self) -> bool {
        self.reader.get_bool_field(224)
      }
      #[inline]
      pub fn get_discriminant_count(&self) -> u16 {
        self.reader.get_data_field::<u16>(15)
      }
      #[inline]
      pub fn get_discriminant_offset(&self) -> u32 {
        self.reader.get_data_field::<u32>(8)
      }
      #[inline]
      pub fn get_fields(&self) -> struct_list::Reader<'a,schema_capnp::Field::Reader<'a>> {
        struct_list::Reader::new(self.reader.get_pointer_field(3).get_list(schema_capnp::Field::STRUCT_SIZE.preferred_list_encoding, std::ptr::null()))
      }
      pub fn has_fields(&self) -> bool {
        !self.reader.get_pointer_field(3).is_null()
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn get_data_word_count(&self) -> u16 {
        self.builder.get_data_field::<u16>(7)
      }
      #[inline]
      pub fn set_data_word_count(&self, value : u16) {
        self.builder.set_data_field::<u16>(7, value);
      }
      #[inline]
      pub fn get_pointer_count(&self) -> u16 {
        self.builder.get_data_field::<u16>(12)
      }
      #[inline]
      pub fn set_pointer_count(&self, value : u16) {
        self.builder.set_data_field::<u16>(12, value);
      }
      #[inline]
      pub fn get_preferred_list_encoding(&self) -> Option<schema_capnp::ElementSize::Reader> {
        FromPrimitive::from_u16(self.builder.get_data_field::<u16>(13))
      }
      #[inline]
      pub fn set_preferred_list_encoding(&self, value : schema_capnp::ElementSize::Reader) {
        self.builder.set_data_field::<u16>(13, value as u16)
      }
      #[inline]
      pub fn get_is_group(&self) -> bool {
        self.builder.get_bool_field(224)
      }
      #[inline]
      pub fn set_is_group(&self, value : bool) {
        self.builder.set_bool_field(224, value);
      }
      #[inline]
      pub fn get_discriminant_count(&self) -> u16 {
        self.builder.get_data_field::<u16>(15)
      }
      #[inline]
      pub fn set_discriminant_count(&self, value : u16) {
        self.builder.set_data_field::<u16>(15, value);
      }
      #[inline]
      pub fn get_discriminant_offset(&self) -> u32 {
        self.builder.get_data_field::<u32>(8)
      }
      #[inline]
      pub fn set_discriminant_offset(&self, value : u32) {
        self.builder.set_data_field::<u32>(8, value);
      }
      #[inline]
      pub fn get_fields(&self) -> struct_list::Builder<'a,schema_capnp::Field::Builder<'a>> {
        struct_list::Builder::new(self.builder.get_pointer_field(3).get_struct_list(schema_capnp::Field::STRUCT_SIZE, std::ptr::null()))
      }
      #[inline]
      pub fn set_fields(&self, value : struct_list::Reader<'a,schema_capnp::Field::Reader<'a>>) {
        self.builder.get_pointer_field(3).set_list(&value.reader)
      }
      #[inline]
      pub fn init_fields(&self, size : uint) -> struct_list::Builder<'a,schema_capnp::Field::Builder<'a>> {
        struct_list::Builder::<'a, schema_capnp::Field::Builder<'a>>::new(
          self.builder.get_pointer_field(3).init_struct_list(size, schema_capnp::Field::STRUCT_SIZE))
      }
      pub fn has_fields(&self) -> bool {
        !self.builder.get_pointer_field(3).is_null()
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
    }
  }

  pub mod Enum {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn get_enumerants(&self) -> struct_list::Reader<'a,schema_capnp::Enumerant::Reader<'a>> {
        struct_list::Reader::new(self.reader.get_pointer_field(3).get_list(schema_capnp::Enumerant::STRUCT_SIZE.preferred_list_encoding, std::ptr::null()))
      }
      pub fn has_enumerants(&self) -> bool {
        !self.reader.get_pointer_field(3).is_null()
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn get_enumerants(&self) -> struct_list::Builder<'a,schema_capnp::Enumerant::Builder<'a>> {
        struct_list::Builder::new(self.builder.get_pointer_field(3).get_struct_list(schema_capnp::Enumerant::STRUCT_SIZE, std::ptr::null()))
      }
      #[inline]
      pub fn set_enumerants(&self, value : struct_list::Reader<'a,schema_capnp::Enumerant::Reader<'a>>) {
        self.builder.get_pointer_field(3).set_list(&value.reader)
      }
      #[inline]
      pub fn init_enumerants(&self, size : uint) -> struct_list::Builder<'a,schema_capnp::Enumerant::Builder<'a>> {
        struct_list::Builder::<'a, schema_capnp::Enumerant::Builder<'a>>::new(
          self.builder.get_pointer_field(3).init_struct_list(size, schema_capnp::Enumerant::STRUCT_SIZE))
      }
      pub fn has_enumerants(&self) -> bool {
        !self.builder.get_pointer_field(3).is_null()
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
    }
  }

  pub mod Interface {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn get_methods(&self) -> struct_list::Reader<'a,schema_capnp::Method::Reader<'a>> {
        struct_list::Reader::new(self.reader.get_pointer_field(3).get_list(schema_capnp::Method::STRUCT_SIZE.preferred_list_encoding, std::ptr::null()))
      }
      pub fn has_methods(&self) -> bool {
        !self.reader.get_pointer_field(3).is_null()
      }
      #[inline]
      pub fn get_extends(&self) -> primitive_list::Reader<'a,u64> {
        primitive_list::Reader::new(self.reader.get_pointer_field(4).get_list(layout::EightBytes, std::ptr::null()))
      }
      pub fn has_extends(&self) -> bool {
        !self.reader.get_pointer_field(4).is_null()
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn get_methods(&self) -> struct_list::Builder<'a,schema_capnp::Method::Builder<'a>> {
        struct_list::Builder::new(self.builder.get_pointer_field(3).get_struct_list(schema_capnp::Method::STRUCT_SIZE, std::ptr::null()))
      }
      #[inline]
      pub fn set_methods(&self, value : struct_list::Reader<'a,schema_capnp::Method::Reader<'a>>) {
        self.builder.get_pointer_field(3).set_list(&value.reader)
      }
      #[inline]
      pub fn init_methods(&self, size : uint) -> struct_list::Builder<'a,schema_capnp::Method::Builder<'a>> {
        struct_list::Builder::<'a, schema_capnp::Method::Builder<'a>>::new(
          self.builder.get_pointer_field(3).init_struct_list(size, schema_capnp::Method::STRUCT_SIZE))
      }
      pub fn has_methods(&self) -> bool {
        !self.builder.get_pointer_field(3).is_null()
      }
      #[inline]
      pub fn get_extends(&self) -> primitive_list::Builder<'a,u64> {
        primitive_list::Builder::new(self.builder.get_pointer_field(4).get_list(layout::EightBytes, std::ptr::null()))
      }
      #[inline]
      pub fn set_extends(&self, value : primitive_list::Reader<'a,u64>) {
        self.builder.get_pointer_field(4).set_list(&value.reader)
      }
      #[inline]
      pub fn init_extends(&self, size : uint) -> primitive_list::Builder<'a,u64> {
        primitive_list::Builder::<'a,u64>::new(
          self.builder.get_pointer_field(4).init_list(layout::EightBytes,size)
        )
      }
      pub fn has_extends(&self) -> bool {
        !self.builder.get_pointer_field(4).is_null()
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
    }
  }

  pub mod Const {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn get_type(&self) -> schema_capnp::Type::Reader<'a> {
        FromStructReader::new(self.reader.get_pointer_field(3).get_struct( std::ptr::null()))
      }
      pub fn has_type(&self) -> bool {
        !self.reader.get_pointer_field(3).is_null()
      }
      #[inline]
      pub fn get_value(&self) -> schema_capnp::Value::Reader<'a> {
        FromStructReader::new(self.reader.get_pointer_field(4).get_struct( std::ptr::null()))
      }
      pub fn has_value(&self) -> bool {
        !self.reader.get_pointer_field(4).is_null()
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn get_type(&self) -> schema_capnp::Type::Builder<'a> {
        FromStructBuilder::new(self.builder.get_pointer_field(3).get_struct(schema_capnp::Type::STRUCT_SIZE, std::ptr::null()))
      }
      #[inline]
      pub fn set_type(&self, value : schema_capnp::Type::Reader) {
        self.builder.get_pointer_field(3).set_struct(&value.struct_reader())
      }
      #[inline]
      pub fn init_type(&self, ) -> schema_capnp::Type::Builder<'a> {
        FromStructBuilder::new(self.builder.get_pointer_field(3).init_struct(schema_capnp::Type::STRUCT_SIZE))
      }
      pub fn has_type(&self) -> bool {
        !self.builder.get_pointer_field(3).is_null()
      }
      #[inline]
      pub fn get_value(&self) -> schema_capnp::Value::Builder<'a> {
        FromStructBuilder::new(self.builder.get_pointer_field(4).get_struct(schema_capnp::Value::STRUCT_SIZE, std::ptr::null()))
      }
      #[inline]
      pub fn set_value(&self, value : schema_capnp::Value::Reader) {
        self.builder.get_pointer_field(4).set_struct(&value.struct_reader())
      }
      #[inline]
      pub fn init_value(&self, ) -> schema_capnp::Value::Builder<'a> {
        FromStructBuilder::new(self.builder.get_pointer_field(4).init_struct(schema_capnp::Value::STRUCT_SIZE))
      }
      pub fn has_value(&self) -> bool {
        !self.builder.get_pointer_field(4).is_null()
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
      pub fn get_type(&self) -> schema_capnp::Type::Pipeline {
        FromTypelessPipeline::new(self._typeless.get_pointer_field(3))
      }
      pub fn get_value(&self) -> schema_capnp::Value::Pipeline {
        FromTypelessPipeline::new(self._typeless.get_pointer_field(4))
      }
    }
  }

  pub mod Annotation {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn get_type(&self) -> schema_capnp::Type::Reader<'a> {
        FromStructReader::new(self.reader.get_pointer_field(3).get_struct( std::ptr::null()))
      }
      pub fn has_type(&self) -> bool {
        !self.reader.get_pointer_field(3).is_null()
      }
      #[inline]
      pub fn get_targets_file(&self) -> bool {
        self.reader.get_bool_field(112)
      }
      #[inline]
      pub fn get_targets_const(&self) -> bool {
        self.reader.get_bool_field(113)
      }
      #[inline]
      pub fn get_targets_enum(&self) -> bool {
        self.reader.get_bool_field(114)
      }
      #[inline]
      pub fn get_targets_enumerant(&self) -> bool {
        self.reader.get_bool_field(115)
      }
      #[inline]
      pub fn get_targets_struct(&self) -> bool {
        self.reader.get_bool_field(116)
      }
      #[inline]
      pub fn get_targets_field(&self) -> bool {
        self.reader.get_bool_field(117)
      }
      #[inline]
      pub fn get_targets_union(&self) -> bool {
        self.reader.get_bool_field(118)
      }
      #[inline]
      pub fn get_targets_group(&self) -> bool {
        self.reader.get_bool_field(119)
      }
      #[inline]
      pub fn get_targets_interface(&self) -> bool {
        self.reader.get_bool_field(120)
      }
      #[inline]
      pub fn get_targets_method(&self) -> bool {
        self.reader.get_bool_field(121)
      }
      #[inline]
      pub fn get_targets_param(&self) -> bool {
        self.reader.get_bool_field(122)
      }
      #[inline]
      pub fn get_targets_annotation(&self) -> bool {
        self.reader.get_bool_field(123)
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn get_type(&self) -> schema_capnp::Type::Builder<'a> {
        FromStructBuilder::new(self.builder.get_pointer_field(3).get_struct(schema_capnp::Type::STRUCT_SIZE, std::ptr::null()))
      }
      #[inline]
      pub fn set_type(&self, value : schema_capnp::Type::Reader) {
        self.builder.get_pointer_field(3).set_struct(&value.struct_reader())
      }
      #[inline]
      pub fn init_type(&self, ) -> schema_capnp::Type::Builder<'a> {
        FromStructBuilder::new(self.builder.get_pointer_field(3).init_struct(schema_capnp::Type::STRUCT_SIZE))
      }
      pub fn has_type(&self) -> bool {
        !self.builder.get_pointer_field(3).is_null()
      }
      #[inline]
      pub fn get_targets_file(&self) -> bool {
        self.builder.get_bool_field(112)
      }
      #[inline]
      pub fn set_targets_file(&self, value : bool) {
        self.builder.set_bool_field(112, value);
      }
      #[inline]
      pub fn get_targets_const(&self) -> bool {
        self.builder.get_bool_field(113)
      }
      #[inline]
      pub fn set_targets_const(&self, value : bool) {
        self.builder.set_bool_field(113, value);
      }
      #[inline]
      pub fn get_targets_enum(&self) -> bool {
        self.builder.get_bool_field(114)
      }
      #[inline]
      pub fn set_targets_enum(&self, value : bool) {
        self.builder.set_bool_field(114, value);
      }
      #[inline]
      pub fn get_targets_enumerant(&self) -> bool {
        self.builder.get_bool_field(115)
      }
      #[inline]
      pub fn set_targets_enumerant(&self, value : bool) {
        self.builder.set_bool_field(115, value);
      }
      #[inline]
      pub fn get_targets_struct(&self) -> bool {
        self.builder.get_bool_field(116)
      }
      #[inline]
      pub fn set_targets_struct(&self, value : bool) {
        self.builder.set_bool_field(116, value);
      }
      #[inline]
      pub fn get_targets_field(&self) -> bool {
        self.builder.get_bool_field(117)
      }
      #[inline]
      pub fn set_targets_field(&self, value : bool) {
        self.builder.set_bool_field(117, value);
      }
      #[inline]
      pub fn get_targets_union(&self) -> bool {
        self.builder.get_bool_field(118)
      }
      #[inline]
      pub fn set_targets_union(&self, value : bool) {
        self.builder.set_bool_field(118, value);
      }
      #[inline]
      pub fn get_targets_group(&self) -> bool {
        self.builder.get_bool_field(119)
      }
      #[inline]
      pub fn set_targets_group(&self, value : bool) {
        self.builder.set_bool_field(119, value);
      }
      #[inline]
      pub fn get_targets_interface(&self) -> bool {
        self.builder.get_bool_field(120)
      }
      #[inline]
      pub fn set_targets_interface(&self, value : bool) {
        self.builder.set_bool_field(120, value);
      }
      #[inline]
      pub fn get_targets_method(&self) -> bool {
        self.builder.get_bool_field(121)
      }
      #[inline]
      pub fn set_targets_method(&self, value : bool) {
        self.builder.set_bool_field(121, value);
      }
      #[inline]
      pub fn get_targets_param(&self) -> bool {
        self.builder.get_bool_field(122)
      }
      #[inline]
      pub fn set_targets_param(&self, value : bool) {
        self.builder.set_bool_field(122, value);
      }
      #[inline]
      pub fn get_targets_annotation(&self) -> bool {
        self.builder.get_bool_field(123)
      }
      #[inline]
      pub fn set_targets_annotation(&self, value : bool) {
        self.builder.set_bool_field(123, value);
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
      pub fn get_type(&self) -> schema_capnp::Type::Pipeline {
        FromTypelessPipeline::new(self._typeless.get_pointer_field(3))
      }
    }
  }
}

pub mod Field {
  use std;
  use capnp::any::any_pointer;
  use capnp::capability::{FromClientHook, FromTypelessPipeline};
  use capnp::blob::{text, data};
  use capnp::layout;
  use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
  use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
  use schema_capnp;

  pub static STRUCT_SIZE : layout::StructSize =
    layout::StructSize { data : 3, pointers : 4, preferred_list_encoding : layout::InlineComposite};


  pub struct Reader<'a> { reader : layout::StructReader<'a> }

  impl <'a> layout::FromStructReader<'a> for Reader<'a> {
    fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
      Reader { reader : reader }
    }
  }

  impl <'a> layout::ToStructReader<'a> for Reader<'a> {
    fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
  }

  impl <'a> Reader<'a> {
    #[inline]
    pub fn get_name(&self) -> text::Reader<'a> {
      self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
    }
    pub fn has_name(&self) -> bool {
      !self.reader.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn get_code_order(&self) -> u16 {
      self.reader.get_data_field::<u16>(0)
    }
    #[inline]
    pub fn get_annotations(&self) -> struct_list::Reader<'a,schema_capnp::Annotation::Reader<'a>> {
      struct_list::Reader::new(self.reader.get_pointer_field(1).get_list(schema_capnp::Annotation::STRUCT_SIZE.preferred_list_encoding, std::ptr::null()))
    }
    pub fn has_annotations(&self) -> bool {
      !self.reader.get_pointer_field(1).is_null()
    }
    #[inline]
    pub fn get_discriminant_value(&self) -> u16 {
      self.reader.get_data_field_mask::<u16>(1, 65535u16)
    }
    #[inline]
    pub fn get_ordinal(&self) -> schema_capnp::Field::Ordinal::Reader<'a> {
      FromStructReader::new(self.reader)
    }
    #[inline]
    pub fn which(&self) -> std::option::Option<WhichReader<'a>> {
      match self.reader.get_data_field::<u16>(4) {
        0 => {
          return std::option::Some(Slot(
            FromStructReader::new(self.reader)
          ));
        }
        1 => {
          return std::option::Some(Group(
            FromStructReader::new(self.reader)
          ));
        }
        _ => return std::option::None
      }
    }
  }

  pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
  impl <'a> layout::HasStructSize for Builder<'a> {
    #[inline]
    fn struct_size(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }
  }
  impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
    fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
      Builder { builder : builder }
    }
  }
  impl <'a> Builder<'a> {
    pub fn as_reader(&self) -> Reader<'a> {
      FromStructReader::new(self.builder.as_reader())
    }
    #[inline]
    pub fn get_name(&self) -> text::Builder<'a> {
      self.builder.get_pointer_field(0).get_text(std::ptr::null(), 0)
    }
    #[inline]
    pub fn set_name(&self, value : text::Reader) {
      self.builder.get_pointer_field(0).set_text(value);
    }
    #[inline]
    pub fn init_name(&self, size : uint) -> text::Builder<'a> {
      self.builder.get_pointer_field(0).init_text(size)
    }
    pub fn has_name(&self) -> bool {
      !self.builder.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn get_code_order(&self) -> u16 {
      self.builder.get_data_field::<u16>(0)
    }
    #[inline]
    pub fn set_code_order(&self, value : u16) {
      self.builder.set_data_field::<u16>(0, value);
    }
    #[inline]
    pub fn get_annotations(&self) -> struct_list::Builder<'a,schema_capnp::Annotation::Builder<'a>> {
      struct_list::Builder::new(self.builder.get_pointer_field(1).get_struct_list(schema_capnp::Annotation::STRUCT_SIZE, std::ptr::null()))
    }
    #[inline]
    pub fn set_annotations(&self, value : struct_list::Reader<'a,schema_capnp::Annotation::Reader<'a>>) {
      self.builder.get_pointer_field(1).set_list(&value.reader)
    }
    #[inline]
    pub fn init_annotations(&self, size : uint) -> struct_list::Builder<'a,schema_capnp::Annotation::Builder<'a>> {
      struct_list::Builder::<'a, schema_capnp::Annotation::Builder<'a>>::new(
        self.builder.get_pointer_field(1).init_struct_list(size, schema_capnp::Annotation::STRUCT_SIZE))
    }
    pub fn has_annotations(&self) -> bool {
      !self.builder.get_pointer_field(1).is_null()
    }
    #[inline]
    pub fn get_discriminant_value(&self) -> u16 {
      self.builder.get_data_field_mask::<u16>(1, 65535u16)
    }
    #[inline]
    pub fn set_discriminant_value(&self, value : u16) {
      self.builder.set_data_field_mask::<u16>(1, value, 65535);
    }
    #[inline]
    pub fn init_slot(&self, ) -> schema_capnp::Field::Slot::Builder<'a> {
      self.builder.set_data_field::<u16>(4, 0);
      self.builder.set_data_field::<u32>(1, 0);
      self.builder.get_pointer_field(2).clear();
      self.builder.get_pointer_field(3).clear();
      self.builder.set_bool_field(128, false);
      FromStructBuilder::new(self.builder)
    }
    #[inline]
    pub fn init_group(&self, ) -> schema_capnp::Field::Group::Builder<'a> {
      self.builder.set_data_field::<u16>(4, 1);
      self.builder.set_data_field::<u64>(2, 0);
      FromStructBuilder::new(self.builder)
    }
    #[inline]
    pub fn get_ordinal(&self) -> schema_capnp::Field::Ordinal::Builder<'a> {
      FromStructBuilder::new(self.builder)
    }
    #[inline]
    pub fn init_ordinal(&self, ) -> schema_capnp::Field::Ordinal::Builder<'a> {
      self.builder.set_data_field::<u16>(5, 0);
      self.builder.set_data_field::<u16>(6, 0);
      FromStructBuilder::new(self.builder)
    }
    #[inline]
    pub fn which(&self) -> std::option::Option<WhichBuilder<'a>> {
      match self.builder.get_data_field::<u16>(4) {
        0 => {
          return std::option::Some(Slot(
            FromStructBuilder::new(self.builder)
          ));
        }
        1 => {
          return std::option::Some(Group(
            FromStructBuilder::new(self.builder)
          ));
        }
        _ => return std::option::None
      }
    }
  }

  pub struct Pipeline { _typeless : any_pointer::Pipeline }
  impl FromTypelessPipeline for Pipeline {
    fn new(typeless : any_pointer::Pipeline) -> Pipeline {
      Pipeline { _typeless : typeless }
    }
  }
  impl Pipeline {
    pub fn get_ordinal(&self) -> schema_capnp::Field::Ordinal::Pipeline {
      FromTypelessPipeline::new(self._typeless.noop())
    }
  }
  pub enum Which<'a,A0,A1> {
    Slot(A0),
    Group(A1),
  }
  pub type WhichReader<'a> = Which<'a,schema_capnp::Field::Slot::Reader<'a>,schema_capnp::Field::Group::Reader<'a>>;
  pub type WhichBuilder<'a> = Which<'a,schema_capnp::Field::Slot::Builder<'a>,schema_capnp::Field::Group::Builder<'a>>;
  pub static NO_DISCRIMINANT : u16 = 65535;

  pub mod Slot {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn get_offset(&self) -> u32 {
        self.reader.get_data_field::<u32>(1)
      }
      #[inline]
      pub fn get_type(&self) -> schema_capnp::Type::Reader<'a> {
        FromStructReader::new(self.reader.get_pointer_field(2).get_struct( std::ptr::null()))
      }
      pub fn has_type(&self) -> bool {
        !self.reader.get_pointer_field(2).is_null()
      }
      #[inline]
      pub fn get_default_value(&self) -> schema_capnp::Value::Reader<'a> {
        FromStructReader::new(self.reader.get_pointer_field(3).get_struct( std::ptr::null()))
      }
      pub fn has_default_value(&self) -> bool {
        !self.reader.get_pointer_field(3).is_null()
      }
      #[inline]
      pub fn get_had_explicit_default(&self) -> bool {
        self.reader.get_bool_field(128)
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn get_offset(&self) -> u32 {
        self.builder.get_data_field::<u32>(1)
      }
      #[inline]
      pub fn set_offset(&self, value : u32) {
        self.builder.set_data_field::<u32>(1, value);
      }
      #[inline]
      pub fn get_type(&self) -> schema_capnp::Type::Builder<'a> {
        FromStructBuilder::new(self.builder.get_pointer_field(2).get_struct(schema_capnp::Type::STRUCT_SIZE, std::ptr::null()))
      }
      #[inline]
      pub fn set_type(&self, value : schema_capnp::Type::Reader) {
        self.builder.get_pointer_field(2).set_struct(&value.struct_reader())
      }
      #[inline]
      pub fn init_type(&self, ) -> schema_capnp::Type::Builder<'a> {
        FromStructBuilder::new(self.builder.get_pointer_field(2).init_struct(schema_capnp::Type::STRUCT_SIZE))
      }
      pub fn has_type(&self) -> bool {
        !self.builder.get_pointer_field(2).is_null()
      }
      #[inline]
      pub fn get_default_value(&self) -> schema_capnp::Value::Builder<'a> {
        FromStructBuilder::new(self.builder.get_pointer_field(3).get_struct(schema_capnp::Value::STRUCT_SIZE, std::ptr::null()))
      }
      #[inline]
      pub fn set_default_value(&self, value : schema_capnp::Value::Reader) {
        self.builder.get_pointer_field(3).set_struct(&value.struct_reader())
      }
      #[inline]
      pub fn init_default_value(&self, ) -> schema_capnp::Value::Builder<'a> {
        FromStructBuilder::new(self.builder.get_pointer_field(3).init_struct(schema_capnp::Value::STRUCT_SIZE))
      }
      pub fn has_default_value(&self) -> bool {
        !self.builder.get_pointer_field(3).is_null()
      }
      #[inline]
      pub fn get_had_explicit_default(&self) -> bool {
        self.builder.get_bool_field(128)
      }
      #[inline]
      pub fn set_had_explicit_default(&self, value : bool) {
        self.builder.set_bool_field(128, value);
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
      pub fn get_type(&self) -> schema_capnp::Type::Pipeline {
        FromTypelessPipeline::new(self._typeless.get_pointer_field(2))
      }
      pub fn get_default_value(&self) -> schema_capnp::Value::Pipeline {
        FromTypelessPipeline::new(self._typeless.get_pointer_field(3))
      }
    }
  }

  pub mod Group {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn get_type_id(&self) -> u64 {
        self.reader.get_data_field::<u64>(2)
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn get_type_id(&self) -> u64 {
        self.builder.get_data_field::<u64>(2)
      }
      #[inline]
      pub fn set_type_id(&self, value : u64) {
        self.builder.set_data_field::<u64>(2, value);
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
    }
  }

  pub mod Ordinal {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn which(&self) -> std::option::Option<WhichReader> {
        match self.reader.get_data_field::<u16>(5) {
          0 => {
            return std::option::Some(Implicit(
              ()
            ));
          }
          1 => {
            return std::option::Some(Explicit(
              self.reader.get_data_field::<u16>(6)
            ));
          }
          _ => return std::option::None
        }
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn set_implicit(&self, _value : ()) {
        self.builder.set_data_field::<u16>(5, 0);
      }
      #[inline]
      pub fn set_explicit(&self, value : u16) {
        self.builder.set_data_field::<u16>(5, 1);
        self.builder.set_data_field::<u16>(6, value);
      }
      #[inline]
      pub fn which(&self) -> std::option::Option<WhichBuilder> {
        match self.builder.get_data_field::<u16>(5) {
          0 => {
            return std::option::Some(Implicit(
              ()
            ));
          }
          1 => {
            return std::option::Some(Explicit(
              self.builder.get_data_field::<u16>(6)
            ));
          }
          _ => return std::option::None
        }
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
    }
    pub enum Which {
      Implicit(()),
      Explicit(u16),
    }
    pub type WhichReader = Which;
    pub type WhichBuilder = Which;
  }
}

pub mod Enumerant {
  use std;
  use capnp::any::any_pointer;
  use capnp::capability::{FromClientHook, FromTypelessPipeline};
  use capnp::blob::{text, data};
  use capnp::layout;
  use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
  use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
  use schema_capnp;

  pub static STRUCT_SIZE : layout::StructSize =
    layout::StructSize { data : 1, pointers : 2, preferred_list_encoding : layout::InlineComposite};


  pub struct Reader<'a> { reader : layout::StructReader<'a> }

  impl <'a> layout::FromStructReader<'a> for Reader<'a> {
    fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
      Reader { reader : reader }
    }
  }

  impl <'a> layout::ToStructReader<'a> for Reader<'a> {
    fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
  }

  impl <'a> Reader<'a> {
    #[inline]
    pub fn get_name(&self) -> text::Reader<'a> {
      self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
    }
    pub fn has_name(&self) -> bool {
      !self.reader.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn get_code_order(&self) -> u16 {
      self.reader.get_data_field::<u16>(0)
    }
    #[inline]
    pub fn get_annotations(&self) -> struct_list::Reader<'a,schema_capnp::Annotation::Reader<'a>> {
      struct_list::Reader::new(self.reader.get_pointer_field(1).get_list(schema_capnp::Annotation::STRUCT_SIZE.preferred_list_encoding, std::ptr::null()))
    }
    pub fn has_annotations(&self) -> bool {
      !self.reader.get_pointer_field(1).is_null()
    }
  }

  pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
  impl <'a> layout::HasStructSize for Builder<'a> {
    #[inline]
    fn struct_size(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }
  }
  impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
    fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
      Builder { builder : builder }
    }
  }
  impl <'a> Builder<'a> {
    pub fn as_reader(&self) -> Reader<'a> {
      FromStructReader::new(self.builder.as_reader())
    }
    #[inline]
    pub fn get_name(&self) -> text::Builder<'a> {
      self.builder.get_pointer_field(0).get_text(std::ptr::null(), 0)
    }
    #[inline]
    pub fn set_name(&self, value : text::Reader) {
      self.builder.get_pointer_field(0).set_text(value);
    }
    #[inline]
    pub fn init_name(&self, size : uint) -> text::Builder<'a> {
      self.builder.get_pointer_field(0).init_text(size)
    }
    pub fn has_name(&self) -> bool {
      !self.builder.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn get_code_order(&self) -> u16 {
      self.builder.get_data_field::<u16>(0)
    }
    #[inline]
    pub fn set_code_order(&self, value : u16) {
      self.builder.set_data_field::<u16>(0, value);
    }
    #[inline]
    pub fn get_annotations(&self) -> struct_list::Builder<'a,schema_capnp::Annotation::Builder<'a>> {
      struct_list::Builder::new(self.builder.get_pointer_field(1).get_struct_list(schema_capnp::Annotation::STRUCT_SIZE, std::ptr::null()))
    }
    #[inline]
    pub fn set_annotations(&self, value : struct_list::Reader<'a,schema_capnp::Annotation::Reader<'a>>) {
      self.builder.get_pointer_field(1).set_list(&value.reader)
    }
    #[inline]
    pub fn init_annotations(&self, size : uint) -> struct_list::Builder<'a,schema_capnp::Annotation::Builder<'a>> {
      struct_list::Builder::<'a, schema_capnp::Annotation::Builder<'a>>::new(
        self.builder.get_pointer_field(1).init_struct_list(size, schema_capnp::Annotation::STRUCT_SIZE))
    }
    pub fn has_annotations(&self) -> bool {
      !self.builder.get_pointer_field(1).is_null()
    }
  }

  pub struct Pipeline { _typeless : any_pointer::Pipeline }
  impl FromTypelessPipeline for Pipeline {
    fn new(typeless : any_pointer::Pipeline) -> Pipeline {
      Pipeline { _typeless : typeless }
    }
  }
  impl Pipeline {
  }
}

pub mod Method {
  use std;
  use capnp::any::any_pointer;
  use capnp::capability::{FromClientHook, FromTypelessPipeline};
  use capnp::blob::{text, data};
  use capnp::layout;
  use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
  use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
  use schema_capnp;

  pub static STRUCT_SIZE : layout::StructSize =
    layout::StructSize { data : 3, pointers : 2, preferred_list_encoding : layout::InlineComposite};


  pub struct Reader<'a> { reader : layout::StructReader<'a> }

  impl <'a> layout::FromStructReader<'a> for Reader<'a> {
    fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
      Reader { reader : reader }
    }
  }

  impl <'a> layout::ToStructReader<'a> for Reader<'a> {
    fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
  }

  impl <'a> Reader<'a> {
    #[inline]
    pub fn get_name(&self) -> text::Reader<'a> {
      self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
    }
    pub fn has_name(&self) -> bool {
      !self.reader.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn get_code_order(&self) -> u16 {
      self.reader.get_data_field::<u16>(0)
    }
    #[inline]
    pub fn get_param_struct_type(&self) -> u64 {
      self.reader.get_data_field::<u64>(1)
    }
    #[inline]
    pub fn get_result_struct_type(&self) -> u64 {
      self.reader.get_data_field::<u64>(2)
    }
    #[inline]
    pub fn get_annotations(&self) -> struct_list::Reader<'a,schema_capnp::Annotation::Reader<'a>> {
      struct_list::Reader::new(self.reader.get_pointer_field(1).get_list(schema_capnp::Annotation::STRUCT_SIZE.preferred_list_encoding, std::ptr::null()))
    }
    pub fn has_annotations(&self) -> bool {
      !self.reader.get_pointer_field(1).is_null()
    }
  }

  pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
  impl <'a> layout::HasStructSize for Builder<'a> {
    #[inline]
    fn struct_size(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }
  }
  impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
    fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
      Builder { builder : builder }
    }
  }
  impl <'a> Builder<'a> {
    pub fn as_reader(&self) -> Reader<'a> {
      FromStructReader::new(self.builder.as_reader())
    }
    #[inline]
    pub fn get_name(&self) -> text::Builder<'a> {
      self.builder.get_pointer_field(0).get_text(std::ptr::null(), 0)
    }
    #[inline]
    pub fn set_name(&self, value : text::Reader) {
      self.builder.get_pointer_field(0).set_text(value);
    }
    #[inline]
    pub fn init_name(&self, size : uint) -> text::Builder<'a> {
      self.builder.get_pointer_field(0).init_text(size)
    }
    pub fn has_name(&self) -> bool {
      !self.builder.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn get_code_order(&self) -> u16 {
      self.builder.get_data_field::<u16>(0)
    }
    #[inline]
    pub fn set_code_order(&self, value : u16) {
      self.builder.set_data_field::<u16>(0, value);
    }
    #[inline]
    pub fn get_param_struct_type(&self) -> u64 {
      self.builder.get_data_field::<u64>(1)
    }
    #[inline]
    pub fn set_param_struct_type(&self, value : u64) {
      self.builder.set_data_field::<u64>(1, value);
    }
    #[inline]
    pub fn get_result_struct_type(&self) -> u64 {
      self.builder.get_data_field::<u64>(2)
    }
    #[inline]
    pub fn set_result_struct_type(&self, value : u64) {
      self.builder.set_data_field::<u64>(2, value);
    }
    #[inline]
    pub fn get_annotations(&self) -> struct_list::Builder<'a,schema_capnp::Annotation::Builder<'a>> {
      struct_list::Builder::new(self.builder.get_pointer_field(1).get_struct_list(schema_capnp::Annotation::STRUCT_SIZE, std::ptr::null()))
    }
    #[inline]
    pub fn set_annotations(&self, value : struct_list::Reader<'a,schema_capnp::Annotation::Reader<'a>>) {
      self.builder.get_pointer_field(1).set_list(&value.reader)
    }
    #[inline]
    pub fn init_annotations(&self, size : uint) -> struct_list::Builder<'a,schema_capnp::Annotation::Builder<'a>> {
      struct_list::Builder::<'a, schema_capnp::Annotation::Builder<'a>>::new(
        self.builder.get_pointer_field(1).init_struct_list(size, schema_capnp::Annotation::STRUCT_SIZE))
    }
    pub fn has_annotations(&self) -> bool {
      !self.builder.get_pointer_field(1).is_null()
    }
  }

  pub struct Pipeline { _typeless : any_pointer::Pipeline }
  impl FromTypelessPipeline for Pipeline {
    fn new(typeless : any_pointer::Pipeline) -> Pipeline {
      Pipeline { _typeless : typeless }
    }
  }
  impl Pipeline {
  }
}

pub mod Type {
  use std;
  use capnp::any::any_pointer;
  use capnp::capability::{FromClientHook, FromTypelessPipeline};
  use capnp::blob::{text, data};
  use capnp::layout;
  use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
  use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
  use schema_capnp;

  pub static STRUCT_SIZE : layout::StructSize =
    layout::StructSize { data : 2, pointers : 1, preferred_list_encoding : layout::InlineComposite};


  pub struct Reader<'a> { reader : layout::StructReader<'a> }

  impl <'a> layout::FromStructReader<'a> for Reader<'a> {
    fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
      Reader { reader : reader }
    }
  }

  impl <'a> layout::ToStructReader<'a> for Reader<'a> {
    fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
  }

  impl <'a> Reader<'a> {
    #[inline]
    pub fn which(&self) -> std::option::Option<WhichReader<'a>> {
      match self.reader.get_data_field::<u16>(0) {
        0 => {
          return std::option::Some(Void(
            ()
          ));
        }
        1 => {
          return std::option::Some(Bool(
            ()
          ));
        }
        2 => {
          return std::option::Some(Int8(
            ()
          ));
        }
        3 => {
          return std::option::Some(Int16(
            ()
          ));
        }
        4 => {
          return std::option::Some(Int32(
            ()
          ));
        }
        5 => {
          return std::option::Some(Int64(
            ()
          ));
        }
        6 => {
          return std::option::Some(Uint8(
            ()
          ));
        }
        7 => {
          return std::option::Some(Uint16(
            ()
          ));
        }
        8 => {
          return std::option::Some(Uint32(
            ()
          ));
        }
        9 => {
          return std::option::Some(Uint64(
            ()
          ));
        }
        10 => {
          return std::option::Some(Float32(
            ()
          ));
        }
        11 => {
          return std::option::Some(Float64(
            ()
          ));
        }
        12 => {
          return std::option::Some(Text(
            ()
          ));
        }
        13 => {
          return std::option::Some(Data(
            ()
          ));
        }
        14 => {
          return std::option::Some(List(
            FromStructReader::new(self.reader)
          ));
        }
        15 => {
          return std::option::Some(Enum(
            FromStructReader::new(self.reader)
          ));
        }
        16 => {
          return std::option::Some(Struct(
            FromStructReader::new(self.reader)
          ));
        }
        17 => {
          return std::option::Some(Interface(
            FromStructReader::new(self.reader)
          ));
        }
        18 => {
          return std::option::Some(AnyPointer(
            ()
          ));
        }
        _ => return std::option::None
      }
    }
  }

  pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
  impl <'a> layout::HasStructSize for Builder<'a> {
    #[inline]
    fn struct_size(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }
  }
  impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
    fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
      Builder { builder : builder }
    }
  }
  impl <'a> Builder<'a> {
    pub fn as_reader(&self) -> Reader<'a> {
      FromStructReader::new(self.builder.as_reader())
    }
    #[inline]
    pub fn set_void(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 0);
    }
    #[inline]
    pub fn set_bool(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 1);
    }
    #[inline]
    pub fn set_int8(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 2);
    }
    #[inline]
    pub fn set_int16(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 3);
    }
    #[inline]
    pub fn set_int32(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 4);
    }
    #[inline]
    pub fn set_int64(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 5);
    }
    #[inline]
    pub fn set_uint8(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 6);
    }
    #[inline]
    pub fn set_uint16(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 7);
    }
    #[inline]
    pub fn set_uint32(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 8);
    }
    #[inline]
    pub fn set_uint64(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 9);
    }
    #[inline]
    pub fn set_float32(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 10);
    }
    #[inline]
    pub fn set_float64(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 11);
    }
    #[inline]
    pub fn set_text(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 12);
    }
    #[inline]
    pub fn set_data(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 13);
    }
    #[inline]
    pub fn init_list(&self, ) -> schema_capnp::Type::List::Builder<'a> {
      self.builder.set_data_field::<u16>(0, 14);
      self.builder.get_pointer_field(0).clear();
      FromStructBuilder::new(self.builder)
    }
    #[inline]
    pub fn init_enum(&self, ) -> schema_capnp::Type::Enum::Builder<'a> {
      self.builder.set_data_field::<u16>(0, 15);
      self.builder.set_data_field::<u64>(1, 0);
      FromStructBuilder::new(self.builder)
    }
    #[inline]
    pub fn init_struct(&self, ) -> schema_capnp::Type::Struct::Builder<'a> {
      self.builder.set_data_field::<u16>(0, 16);
      self.builder.set_data_field::<u64>(1, 0);
      FromStructBuilder::new(self.builder)
    }
    #[inline]
    pub fn init_interface(&self, ) -> schema_capnp::Type::Interface::Builder<'a> {
      self.builder.set_data_field::<u16>(0, 17);
      self.builder.set_data_field::<u64>(1, 0);
      FromStructBuilder::new(self.builder)
    }
    #[inline]
    pub fn set_any_pointer(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 18);
    }
    #[inline]
    pub fn which(&self) -> std::option::Option<WhichBuilder<'a>> {
      match self.builder.get_data_field::<u16>(0) {
        0 => {
          return std::option::Some(Void(
            ()
          ));
        }
        1 => {
          return std::option::Some(Bool(
            ()
          ));
        }
        2 => {
          return std::option::Some(Int8(
            ()
          ));
        }
        3 => {
          return std::option::Some(Int16(
            ()
          ));
        }
        4 => {
          return std::option::Some(Int32(
            ()
          ));
        }
        5 => {
          return std::option::Some(Int64(
            ()
          ));
        }
        6 => {
          return std::option::Some(Uint8(
            ()
          ));
        }
        7 => {
          return std::option::Some(Uint16(
            ()
          ));
        }
        8 => {
          return std::option::Some(Uint32(
            ()
          ));
        }
        9 => {
          return std::option::Some(Uint64(
            ()
          ));
        }
        10 => {
          return std::option::Some(Float32(
            ()
          ));
        }
        11 => {
          return std::option::Some(Float64(
            ()
          ));
        }
        12 => {
          return std::option::Some(Text(
            ()
          ));
        }
        13 => {
          return std::option::Some(Data(
            ()
          ));
        }
        14 => {
          return std::option::Some(List(
            FromStructBuilder::new(self.builder)
          ));
        }
        15 => {
          return std::option::Some(Enum(
            FromStructBuilder::new(self.builder)
          ));
        }
        16 => {
          return std::option::Some(Struct(
            FromStructBuilder::new(self.builder)
          ));
        }
        17 => {
          return std::option::Some(Interface(
            FromStructBuilder::new(self.builder)
          ));
        }
        18 => {
          return std::option::Some(AnyPointer(
            ()
          ));
        }
        _ => return std::option::None
      }
    }
  }

  pub struct Pipeline { _typeless : any_pointer::Pipeline }
  impl FromTypelessPipeline for Pipeline {
    fn new(typeless : any_pointer::Pipeline) -> Pipeline {
      Pipeline { _typeless : typeless }
    }
  }
  impl Pipeline {
  }
  pub enum Which<'a,A0,A1,A2,A3> {
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
    List(A0),
    Enum(A1),
    Struct(A2),
    Interface(A3),
    AnyPointer(()),
  }
  pub type WhichReader<'a> = Which<'a,schema_capnp::Type::List::Reader<'a>,schema_capnp::Type::Enum::Reader<'a>,schema_capnp::Type::Struct::Reader<'a>,schema_capnp::Type::Interface::Reader<'a>>;
  pub type WhichBuilder<'a> = Which<'a,schema_capnp::Type::List::Builder<'a>,schema_capnp::Type::Enum::Builder<'a>,schema_capnp::Type::Struct::Builder<'a>,schema_capnp::Type::Interface::Builder<'a>>;

  pub mod List {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn get_element_type(&self) -> schema_capnp::Type::Reader<'a> {
        FromStructReader::new(self.reader.get_pointer_field(0).get_struct( std::ptr::null()))
      }
      pub fn has_element_type(&self) -> bool {
        !self.reader.get_pointer_field(0).is_null()
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn get_element_type(&self) -> schema_capnp::Type::Builder<'a> {
        FromStructBuilder::new(self.builder.get_pointer_field(0).get_struct(schema_capnp::Type::STRUCT_SIZE, std::ptr::null()))
      }
      #[inline]
      pub fn set_element_type(&self, value : schema_capnp::Type::Reader) {
        self.builder.get_pointer_field(0).set_struct(&value.struct_reader())
      }
      #[inline]
      pub fn init_element_type(&self, ) -> schema_capnp::Type::Builder<'a> {
        FromStructBuilder::new(self.builder.get_pointer_field(0).init_struct(schema_capnp::Type::STRUCT_SIZE))
      }
      pub fn has_element_type(&self) -> bool {
        !self.builder.get_pointer_field(0).is_null()
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
      pub fn get_element_type(&self) -> schema_capnp::Type::Pipeline {
        FromTypelessPipeline::new(self._typeless.get_pointer_field(0))
      }
    }
  }

  pub mod Enum {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn get_type_id(&self) -> u64 {
        self.reader.get_data_field::<u64>(1)
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn get_type_id(&self) -> u64 {
        self.builder.get_data_field::<u64>(1)
      }
      #[inline]
      pub fn set_type_id(&self, value : u64) {
        self.builder.set_data_field::<u64>(1, value);
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
    }
  }

  pub mod Struct {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn get_type_id(&self) -> u64 {
        self.reader.get_data_field::<u64>(1)
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn get_type_id(&self) -> u64 {
        self.builder.get_data_field::<u64>(1)
      }
      #[inline]
      pub fn set_type_id(&self, value : u64) {
        self.builder.set_data_field::<u64>(1, value);
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
    }
  }

  pub mod Interface {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn get_type_id(&self) -> u64 {
        self.reader.get_data_field::<u64>(1)
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn get_type_id(&self) -> u64 {
        self.builder.get_data_field::<u64>(1)
      }
      #[inline]
      pub fn set_type_id(&self, value : u64) {
        self.builder.set_data_field::<u64>(1, value);
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
    }
  }
}

pub mod Value {
  use std;
  use capnp::any::any_pointer;
  use capnp::capability::{FromClientHook, FromTypelessPipeline};
  use capnp::blob::{text, data};
  use capnp::layout;
  use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
  use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
  use schema_capnp;

  pub static STRUCT_SIZE : layout::StructSize =
    layout::StructSize { data : 2, pointers : 1, preferred_list_encoding : layout::InlineComposite};


  pub struct Reader<'a> { reader : layout::StructReader<'a> }

  impl <'a> layout::FromStructReader<'a> for Reader<'a> {
    fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
      Reader { reader : reader }
    }
  }

  impl <'a> layout::ToStructReader<'a> for Reader<'a> {
    fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
  }

  impl <'a> Reader<'a> {
    pub fn has_text(&self) -> bool {
      if self.reader.get_data_field::<u16>(0) != 12 { return false; }
      !self.reader.get_pointer_field(0).is_null()
    }
    pub fn has_data(&self) -> bool {
      if self.reader.get_data_field::<u16>(0) != 13 { return false; }
      !self.reader.get_pointer_field(0).is_null()
    }
    pub fn has_list(&self) -> bool {
      if self.reader.get_data_field::<u16>(0) != 14 { return false; }
      !self.reader.get_pointer_field(0).is_null()
    }
    pub fn has_struct(&self) -> bool {
      if self.reader.get_data_field::<u16>(0) != 16 { return false; }
      !self.reader.get_pointer_field(0).is_null()
    }
    pub fn has_any_pointer(&self) -> bool {
      if self.reader.get_data_field::<u16>(0) != 18 { return false; }
      !self.reader.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn which(&self) -> std::option::Option<WhichReader<'a>> {
      match self.reader.get_data_field::<u16>(0) {
        0 => {
          return std::option::Some(Void(
            ()
          ));
        }
        1 => {
          return std::option::Some(Bool(
            self.reader.get_bool_field(16)
          ));
        }
        2 => {
          return std::option::Some(Int8(
            self.reader.get_data_field::<i8>(2)
          ));
        }
        3 => {
          return std::option::Some(Int16(
            self.reader.get_data_field::<i16>(1)
          ));
        }
        4 => {
          return std::option::Some(Int32(
            self.reader.get_data_field::<i32>(1)
          ));
        }
        5 => {
          return std::option::Some(Int64(
            self.reader.get_data_field::<i64>(1)
          ));
        }
        6 => {
          return std::option::Some(Uint8(
            self.reader.get_data_field::<u8>(2)
          ));
        }
        7 => {
          return std::option::Some(Uint16(
            self.reader.get_data_field::<u16>(1)
          ));
        }
        8 => {
          return std::option::Some(Uint32(
            self.reader.get_data_field::<u32>(1)
          ));
        }
        9 => {
          return std::option::Some(Uint64(
            self.reader.get_data_field::<u64>(1)
          ));
        }
        10 => {
          return std::option::Some(Float32(
            self.reader.get_data_field::<f32>(1)
          ));
        }
        11 => {
          return std::option::Some(Float64(
            self.reader.get_data_field::<f64>(1)
          ));
        }
        12 => {
          return std::option::Some(Text(
            self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
          ));
        }
        13 => {
          return std::option::Some(Data(
            self.reader.get_pointer_field(0).get_data(std::ptr::null(), 0)
          ));
        }
        14 => {
          return std::option::Some(List(
            any_pointer::Reader::new(self.reader.get_pointer_field(0))
          ));
        }
        15 => {
          return std::option::Some(Enum(
            self.reader.get_data_field::<u16>(1)
          ));
        }
        16 => {
          return std::option::Some(Struct(
            any_pointer::Reader::new(self.reader.get_pointer_field(0))
          ));
        }
        17 => {
          return std::option::Some(Interface(
            ()
          ));
        }
        18 => {
          return std::option::Some(AnyPointer(
            any_pointer::Reader::new(self.reader.get_pointer_field(0))
          ));
        }
        _ => return std::option::None
      }
    }
  }

  pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
  impl <'a> layout::HasStructSize for Builder<'a> {
    #[inline]
    fn struct_size(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }
  }
  impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
    fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
      Builder { builder : builder }
    }
  }
  impl <'a> Builder<'a> {
    pub fn as_reader(&self) -> Reader<'a> {
      FromStructReader::new(self.builder.as_reader())
    }
    #[inline]
    pub fn set_void(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 0);
    }
    #[inline]
    pub fn set_bool(&self, value : bool) {
      self.builder.set_data_field::<u16>(0, 1);
      self.builder.set_bool_field(16, value);
    }
    #[inline]
    pub fn set_int8(&self, value : i8) {
      self.builder.set_data_field::<u16>(0, 2);
      self.builder.set_data_field::<i8>(2, value);
    }
    #[inline]
    pub fn set_int16(&self, value : i16) {
      self.builder.set_data_field::<u16>(0, 3);
      self.builder.set_data_field::<i16>(1, value);
    }
    #[inline]
    pub fn set_int32(&self, value : i32) {
      self.builder.set_data_field::<u16>(0, 4);
      self.builder.set_data_field::<i32>(1, value);
    }
    #[inline]
    pub fn set_int64(&self, value : i64) {
      self.builder.set_data_field::<u16>(0, 5);
      self.builder.set_data_field::<i64>(1, value);
    }
    #[inline]
    pub fn set_uint8(&self, value : u8) {
      self.builder.set_data_field::<u16>(0, 6);
      self.builder.set_data_field::<u8>(2, value);
    }
    #[inline]
    pub fn set_uint16(&self, value : u16) {
      self.builder.set_data_field::<u16>(0, 7);
      self.builder.set_data_field::<u16>(1, value);
    }
    #[inline]
    pub fn set_uint32(&self, value : u32) {
      self.builder.set_data_field::<u16>(0, 8);
      self.builder.set_data_field::<u32>(1, value);
    }
    #[inline]
    pub fn set_uint64(&self, value : u64) {
      self.builder.set_data_field::<u16>(0, 9);
      self.builder.set_data_field::<u64>(1, value);
    }
    #[inline]
    pub fn set_float32(&self, value : f32) {
      self.builder.set_data_field::<u16>(0, 10);
      self.builder.set_data_field::<f32>(1, value);
    }
    #[inline]
    pub fn set_float64(&self, value : f64) {
      self.builder.set_data_field::<u16>(0, 11);
      self.builder.set_data_field::<f64>(1, value);
    }
    #[inline]
    pub fn set_text(&self, value : text::Reader) {
      self.builder.set_data_field::<u16>(0, 12);
      self.builder.get_pointer_field(0).set_text(value);
    }
    #[inline]
    pub fn init_text(&self, size : uint) -> text::Builder<'a> {
      self.builder.set_data_field::<u16>(0, 12);
      self.builder.get_pointer_field(0).init_text(size)
    }
    pub fn has_text(&self) -> bool {
      if self.builder.get_data_field::<u16>(0) != 12 { return false; }
      !self.builder.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn set_data(&self, value : data::Reader) {
      self.builder.set_data_field::<u16>(0, 13);
      self.builder.get_pointer_field(0).set_data(value);
    }
    #[inline]
    pub fn init_data(&self, size : uint) -> data::Builder<'a> {
      self.builder.set_data_field::<u16>(0, 13);
      self.builder.get_pointer_field(0).init_data(size)
    }
    pub fn has_data(&self) -> bool {
      if self.builder.get_data_field::<u16>(0) != 13 { return false; }
      !self.builder.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn init_list(&self, ) -> any_pointer::Builder<'a> {
      self.builder.set_data_field::<u16>(0, 14);
      let result = any_pointer::Builder::new(self.builder.get_pointer_field(0));
      result.clear();
      result
    }
    pub fn has_list(&self) -> bool {
      if self.builder.get_data_field::<u16>(0) != 14 { return false; }
      !self.builder.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn set_enum(&self, value : u16) {
      self.builder.set_data_field::<u16>(0, 15);
      self.builder.set_data_field::<u16>(1, value);
    }
    #[inline]
    pub fn init_struct(&self, ) -> any_pointer::Builder<'a> {
      self.builder.set_data_field::<u16>(0, 16);
      let result = any_pointer::Builder::new(self.builder.get_pointer_field(0));
      result.clear();
      result
    }
    pub fn has_struct(&self) -> bool {
      if self.builder.get_data_field::<u16>(0) != 16 { return false; }
      !self.builder.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn set_interface(&self, _value : ()) {
      self.builder.set_data_field::<u16>(0, 17);
    }
    #[inline]
    pub fn init_any_pointer(&self, ) -> any_pointer::Builder<'a> {
      self.builder.set_data_field::<u16>(0, 18);
      let result = any_pointer::Builder::new(self.builder.get_pointer_field(0));
      result.clear();
      result
    }
    pub fn has_any_pointer(&self) -> bool {
      if self.builder.get_data_field::<u16>(0) != 18 { return false; }
      !self.builder.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn which(&self) -> std::option::Option<WhichBuilder<'a>> {
      match self.builder.get_data_field::<u16>(0) {
        0 => {
          return std::option::Some(Void(
            ()
          ));
        }
        1 => {
          return std::option::Some(Bool(
            self.builder.get_bool_field(16)
          ));
        }
        2 => {
          return std::option::Some(Int8(
            self.builder.get_data_field::<i8>(2)
          ));
        }
        3 => {
          return std::option::Some(Int16(
            self.builder.get_data_field::<i16>(1)
          ));
        }
        4 => {
          return std::option::Some(Int32(
            self.builder.get_data_field::<i32>(1)
          ));
        }
        5 => {
          return std::option::Some(Int64(
            self.builder.get_data_field::<i64>(1)
          ));
        }
        6 => {
          return std::option::Some(Uint8(
            self.builder.get_data_field::<u8>(2)
          ));
        }
        7 => {
          return std::option::Some(Uint16(
            self.builder.get_data_field::<u16>(1)
          ));
        }
        8 => {
          return std::option::Some(Uint32(
            self.builder.get_data_field::<u32>(1)
          ));
        }
        9 => {
          return std::option::Some(Uint64(
            self.builder.get_data_field::<u64>(1)
          ));
        }
        10 => {
          return std::option::Some(Float32(
            self.builder.get_data_field::<f32>(1)
          ));
        }
        11 => {
          return std::option::Some(Float64(
            self.builder.get_data_field::<f64>(1)
          ));
        }
        12 => {
          return std::option::Some(Text(
            self.builder.get_pointer_field(0).get_text(std::ptr::null(), 0)
          ));
        }
        13 => {
          return std::option::Some(Data(
            self.builder.get_pointer_field(0).get_data(std::ptr::null(), 0)
          ));
        }
        14 => {
          return std::option::Some(List(
            any_pointer::Builder::new(self.builder.get_pointer_field(0))
          ));
        }
        15 => {
          return std::option::Some(Enum(
            self.builder.get_data_field::<u16>(1)
          ));
        }
        16 => {
          return std::option::Some(Struct(
            any_pointer::Builder::new(self.builder.get_pointer_field(0))
          ));
        }
        17 => {
          return std::option::Some(Interface(
            ()
          ));
        }
        18 => {
          return std::option::Some(AnyPointer(
            any_pointer::Builder::new(self.builder.get_pointer_field(0))
          ));
        }
        _ => return std::option::None
      }
    }
  }

  pub struct Pipeline { _typeless : any_pointer::Pipeline }
  impl FromTypelessPipeline for Pipeline {
    fn new(typeless : any_pointer::Pipeline) -> Pipeline {
      Pipeline { _typeless : typeless }
    }
  }
  impl Pipeline {
  }
  pub enum Which<'a,A0,A1,A2,A3,A4> {
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
    Text(A0),
    Data(A1),
    List(A2),
    Enum(u16),
    Struct(A3),
    Interface(()),
    AnyPointer(A4),
  }
  pub type WhichReader<'a> = Which<'a,text::Reader<'a>,data::Reader<'a>,any_pointer::Reader<'a>,any_pointer::Reader<'a>,any_pointer::Reader<'a>>;
  pub type WhichBuilder<'a> = Which<'a,text::Builder<'a>,data::Builder<'a>,any_pointer::Builder<'a>,any_pointer::Builder<'a>,any_pointer::Builder<'a>>;
}

pub mod Annotation {
  use std;
  use capnp::any::any_pointer;
  use capnp::capability::{FromClientHook, FromTypelessPipeline};
  use capnp::blob::{text, data};
  use capnp::layout;
  use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
  use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
  use schema_capnp;

  pub static STRUCT_SIZE : layout::StructSize =
    layout::StructSize { data : 1, pointers : 1, preferred_list_encoding : layout::InlineComposite};


  pub struct Reader<'a> { reader : layout::StructReader<'a> }

  impl <'a> layout::FromStructReader<'a> for Reader<'a> {
    fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
      Reader { reader : reader }
    }
  }

  impl <'a> layout::ToStructReader<'a> for Reader<'a> {
    fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
  }

  impl <'a> Reader<'a> {
    #[inline]
    pub fn get_id(&self) -> u64 {
      self.reader.get_data_field::<u64>(0)
    }
    #[inline]
    pub fn get_value(&self) -> schema_capnp::Value::Reader<'a> {
      FromStructReader::new(self.reader.get_pointer_field(0).get_struct( std::ptr::null()))
    }
    pub fn has_value(&self) -> bool {
      !self.reader.get_pointer_field(0).is_null()
    }
  }

  pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
  impl <'a> layout::HasStructSize for Builder<'a> {
    #[inline]
    fn struct_size(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }
  }
  impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
    fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
      Builder { builder : builder }
    }
  }
  impl <'a> Builder<'a> {
    pub fn as_reader(&self) -> Reader<'a> {
      FromStructReader::new(self.builder.as_reader())
    }
    #[inline]
    pub fn get_id(&self) -> u64 {
      self.builder.get_data_field::<u64>(0)
    }
    #[inline]
    pub fn set_id(&self, value : u64) {
      self.builder.set_data_field::<u64>(0, value);
    }
    #[inline]
    pub fn get_value(&self) -> schema_capnp::Value::Builder<'a> {
      FromStructBuilder::new(self.builder.get_pointer_field(0).get_struct(schema_capnp::Value::STRUCT_SIZE, std::ptr::null()))
    }
    #[inline]
    pub fn set_value(&self, value : schema_capnp::Value::Reader) {
      self.builder.get_pointer_field(0).set_struct(&value.struct_reader())
    }
    #[inline]
    pub fn init_value(&self, ) -> schema_capnp::Value::Builder<'a> {
      FromStructBuilder::new(self.builder.get_pointer_field(0).init_struct(schema_capnp::Value::STRUCT_SIZE))
    }
    pub fn has_value(&self) -> bool {
      !self.builder.get_pointer_field(0).is_null()
    }
  }

  pub struct Pipeline { _typeless : any_pointer::Pipeline }
  impl FromTypelessPipeline for Pipeline {
    fn new(typeless : any_pointer::Pipeline) -> Pipeline {
      Pipeline { _typeless : typeless }
    }
  }
  impl Pipeline {
    pub fn get_value(&self) -> schema_capnp::Value::Pipeline {
      FromTypelessPipeline::new(self._typeless.get_pointer_field(0))
    }
  }
}

pub mod ElementSize {
  use capnp::list::{ToU16};

  #[repr(u16)]
  #[deriving(FromPrimitive)]
  #[deriving(PartialEq)]
  pub enum Reader {
    Empty = 0,
    Bit = 1,
    Byte = 2,
    TwoBytes = 3,
    FourBytes = 4,
    EightBytes = 5,
    Pointer = 6,
    InlineComposite = 7,
  }
  impl ToU16 for Reader {
    #[inline]
    fn to_u16(self) -> u16 { self as u16 }
  }
}

pub mod CodeGeneratorRequest {
  use std;
  use capnp::any::any_pointer;
  use capnp::capability::{FromClientHook, FromTypelessPipeline};
  use capnp::blob::{text, data};
  use capnp::layout;
  use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
  use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
  use schema_capnp;

  pub static STRUCT_SIZE : layout::StructSize =
    layout::StructSize { data : 0, pointers : 2, preferred_list_encoding : layout::InlineComposite};


  pub struct Reader<'a> { reader : layout::StructReader<'a> }

  impl <'a> layout::FromStructReader<'a> for Reader<'a> {
    fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
      Reader { reader : reader }
    }
  }

  impl <'a> layout::ToStructReader<'a> for Reader<'a> {
    fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
  }

  impl <'a> Reader<'a> {
    #[inline]
    pub fn get_nodes(&self) -> struct_list::Reader<'a,schema_capnp::Node::Reader<'a>> {
      struct_list::Reader::new(self.reader.get_pointer_field(0).get_list(schema_capnp::Node::STRUCT_SIZE.preferred_list_encoding, std::ptr::null()))
    }
    pub fn has_nodes(&self) -> bool {
      !self.reader.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn get_requested_files(&self) -> struct_list::Reader<'a,schema_capnp::CodeGeneratorRequest::RequestedFile::Reader<'a>> {
      struct_list::Reader::new(self.reader.get_pointer_field(1).get_list(schema_capnp::CodeGeneratorRequest::RequestedFile::STRUCT_SIZE.preferred_list_encoding, std::ptr::null()))
    }
    pub fn has_requested_files(&self) -> bool {
      !self.reader.get_pointer_field(1).is_null()
    }
  }

  pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
  impl <'a> layout::HasStructSize for Builder<'a> {
    #[inline]
    fn struct_size(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }
  }
  impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
    fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
      Builder { builder : builder }
    }
  }
  impl <'a> Builder<'a> {
    pub fn as_reader(&self) -> Reader<'a> {
      FromStructReader::new(self.builder.as_reader())
    }
    #[inline]
    pub fn get_nodes(&self) -> struct_list::Builder<'a,schema_capnp::Node::Builder<'a>> {
      struct_list::Builder::new(self.builder.get_pointer_field(0).get_struct_list(schema_capnp::Node::STRUCT_SIZE, std::ptr::null()))
    }
    #[inline]
    pub fn set_nodes(&self, value : struct_list::Reader<'a,schema_capnp::Node::Reader<'a>>) {
      self.builder.get_pointer_field(0).set_list(&value.reader)
    }
    #[inline]
    pub fn init_nodes(&self, size : uint) -> struct_list::Builder<'a,schema_capnp::Node::Builder<'a>> {
      struct_list::Builder::<'a, schema_capnp::Node::Builder<'a>>::new(
        self.builder.get_pointer_field(0).init_struct_list(size, schema_capnp::Node::STRUCT_SIZE))
    }
    pub fn has_nodes(&self) -> bool {
      !self.builder.get_pointer_field(0).is_null()
    }
    #[inline]
    pub fn get_requested_files(&self) -> struct_list::Builder<'a,schema_capnp::CodeGeneratorRequest::RequestedFile::Builder<'a>> {
      struct_list::Builder::new(self.builder.get_pointer_field(1).get_struct_list(schema_capnp::CodeGeneratorRequest::RequestedFile::STRUCT_SIZE, std::ptr::null()))
    }
    #[inline]
    pub fn set_requested_files(&self, value : struct_list::Reader<'a,schema_capnp::CodeGeneratorRequest::RequestedFile::Reader<'a>>) {
      self.builder.get_pointer_field(1).set_list(&value.reader)
    }
    #[inline]
    pub fn init_requested_files(&self, size : uint) -> struct_list::Builder<'a,schema_capnp::CodeGeneratorRequest::RequestedFile::Builder<'a>> {
      struct_list::Builder::<'a, schema_capnp::CodeGeneratorRequest::RequestedFile::Builder<'a>>::new(
        self.builder.get_pointer_field(1).init_struct_list(size, schema_capnp::CodeGeneratorRequest::RequestedFile::STRUCT_SIZE))
    }
    pub fn has_requested_files(&self) -> bool {
      !self.builder.get_pointer_field(1).is_null()
    }
  }

  pub struct Pipeline { _typeless : any_pointer::Pipeline }
  impl FromTypelessPipeline for Pipeline {
    fn new(typeless : any_pointer::Pipeline) -> Pipeline {
      Pipeline { _typeless : typeless }
    }
  }
  impl Pipeline {
  }

  pub mod RequestedFile {
    use std;
    use capnp::any::any_pointer;
    use capnp::capability::{FromClientHook, FromTypelessPipeline};
    use capnp::blob::{text, data};
    use capnp::layout;
    use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
    use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
    use schema_capnp;

    pub static STRUCT_SIZE : layout::StructSize =
      layout::StructSize { data : 1, pointers : 2, preferred_list_encoding : layout::InlineComposite};


    pub struct Reader<'a> { reader : layout::StructReader<'a> }

    impl <'a> layout::FromStructReader<'a> for Reader<'a> {
      fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
        Reader { reader : reader }
      }
    }

    impl <'a> layout::ToStructReader<'a> for Reader<'a> {
      fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
    }

    impl <'a> Reader<'a> {
      #[inline]
      pub fn get_id(&self) -> u64 {
        self.reader.get_data_field::<u64>(0)
      }
      #[inline]
      pub fn get_filename(&self) -> text::Reader<'a> {
        self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
      }
      pub fn has_filename(&self) -> bool {
        !self.reader.get_pointer_field(0).is_null()
      }
      #[inline]
      pub fn get_imports(&self) -> struct_list::Reader<'a,schema_capnp::CodeGeneratorRequest::RequestedFile::Import::Reader<'a>> {
        struct_list::Reader::new(self.reader.get_pointer_field(1).get_list(schema_capnp::CodeGeneratorRequest::RequestedFile::Import::STRUCT_SIZE.preferred_list_encoding, std::ptr::null()))
      }
      pub fn has_imports(&self) -> bool {
        !self.reader.get_pointer_field(1).is_null()
      }
    }

    pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
    impl <'a> layout::HasStructSize for Builder<'a> {
      #[inline]
      fn struct_size(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }
    }
    impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
      fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
      }
    }
    impl <'a> Builder<'a> {
      pub fn as_reader(&self) -> Reader<'a> {
        FromStructReader::new(self.builder.as_reader())
      }
      #[inline]
      pub fn get_id(&self) -> u64 {
        self.builder.get_data_field::<u64>(0)
      }
      #[inline]
      pub fn set_id(&self, value : u64) {
        self.builder.set_data_field::<u64>(0, value);
      }
      #[inline]
      pub fn get_filename(&self) -> text::Builder<'a> {
        self.builder.get_pointer_field(0).get_text(std::ptr::null(), 0)
      }
      #[inline]
      pub fn set_filename(&self, value : text::Reader) {
        self.builder.get_pointer_field(0).set_text(value);
      }
      #[inline]
      pub fn init_filename(&self, size : uint) -> text::Builder<'a> {
        self.builder.get_pointer_field(0).init_text(size)
      }
      pub fn has_filename(&self) -> bool {
        !self.builder.get_pointer_field(0).is_null()
      }
      #[inline]
      pub fn get_imports(&self) -> struct_list::Builder<'a,schema_capnp::CodeGeneratorRequest::RequestedFile::Import::Builder<'a>> {
        struct_list::Builder::new(self.builder.get_pointer_field(1).get_struct_list(schema_capnp::CodeGeneratorRequest::RequestedFile::Import::STRUCT_SIZE, std::ptr::null()))
      }
      #[inline]
      pub fn set_imports(&self, value : struct_list::Reader<'a,schema_capnp::CodeGeneratorRequest::RequestedFile::Import::Reader<'a>>) {
        self.builder.get_pointer_field(1).set_list(&value.reader)
      }
      #[inline]
      pub fn init_imports(&self, size : uint) -> struct_list::Builder<'a,schema_capnp::CodeGeneratorRequest::RequestedFile::Import::Builder<'a>> {
        struct_list::Builder::<'a, schema_capnp::CodeGeneratorRequest::RequestedFile::Import::Builder<'a>>::new(
          self.builder.get_pointer_field(1).init_struct_list(size, schema_capnp::CodeGeneratorRequest::RequestedFile::Import::STRUCT_SIZE))
      }
      pub fn has_imports(&self) -> bool {
        !self.builder.get_pointer_field(1).is_null()
      }
    }

    pub struct Pipeline { _typeless : any_pointer::Pipeline }
    impl FromTypelessPipeline for Pipeline {
      fn new(typeless : any_pointer::Pipeline) -> Pipeline {
        Pipeline { _typeless : typeless }
      }
    }
    impl Pipeline {
    }

    pub mod Import {
      use std;
      use capnp::any::any_pointer;
      use capnp::capability::{FromClientHook, FromTypelessPipeline};
      use capnp::blob::{text, data};
      use capnp::layout;
      use capnp::layout::{FromStructBuilder, FromStructReader, ToStructReader};
      use capnp::list::{primitive_list, ToU16, enum_list, struct_list, text_list, data_list, list_list};
      use schema_capnp;

      pub static STRUCT_SIZE : layout::StructSize =
        layout::StructSize { data : 1, pointers : 1, preferred_list_encoding : layout::InlineComposite};


      pub struct Reader<'a> { reader : layout::StructReader<'a> }

      impl <'a> layout::FromStructReader<'a> for Reader<'a> {
        fn new(reader: layout::StructReader<'a>) -> Reader<'a> {
          Reader { reader : reader }
        }
      }

      impl <'a> layout::ToStructReader<'a> for Reader<'a> {
        fn struct_reader(&self) -> layout::StructReader<'a> { self.reader }
      }

      impl <'a> Reader<'a> {
        #[inline]
        pub fn get_id(&self) -> u64 {
          self.reader.get_data_field::<u64>(0)
        }
        #[inline]
        pub fn get_name(&self) -> text::Reader<'a> {
          self.reader.get_pointer_field(0).get_text(std::ptr::null(), 0)
        }
        pub fn has_name(&self) -> bool {
          !self.reader.get_pointer_field(0).is_null()
        }
      }

      pub struct Builder<'a> { builder : layout::StructBuilder<'a> }
      impl <'a> layout::HasStructSize for Builder<'a> {
        #[inline]
        fn struct_size(_unused_self : Option<Builder>) -> layout::StructSize { STRUCT_SIZE }
      }
      impl <'a> layout::FromStructBuilder<'a> for Builder<'a> {
        fn new(builder : layout::StructBuilder<'a>) -> Builder<'a> {
          Builder { builder : builder }
        }
      }
      impl <'a> Builder<'a> {
        pub fn as_reader(&self) -> Reader<'a> {
          FromStructReader::new(self.builder.as_reader())
        }
        #[inline]
        pub fn get_id(&self) -> u64 {
          self.builder.get_data_field::<u64>(0)
        }
        #[inline]
        pub fn set_id(&self, value : u64) {
          self.builder.set_data_field::<u64>(0, value);
        }
        #[inline]
        pub fn get_name(&self) -> text::Builder<'a> {
          self.builder.get_pointer_field(0).get_text(std::ptr::null(), 0)
        }
        #[inline]
        pub fn set_name(&self, value : text::Reader) {
          self.builder.get_pointer_field(0).set_text(value);
        }
        #[inline]
        pub fn init_name(&self, size : uint) -> text::Builder<'a> {
          self.builder.get_pointer_field(0).init_text(size)
        }
        pub fn has_name(&self) -> bool {
          !self.builder.get_pointer_field(0).is_null()
        }
      }

      pub struct Pipeline { _typeless : any_pointer::Pipeline }
      impl FromTypelessPipeline for Pipeline {
        fn new(typeless : any_pointer::Pipeline) -> Pipeline {
          Pipeline { _typeless : typeless }
        }
      }
      impl Pipeline {
      }
    }
  }
}
