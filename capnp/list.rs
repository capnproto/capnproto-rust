/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use layout::{PointerBuilder, PointerReader};
use common::Word;

pub trait FromPointerReader<'a> {
    fn get_from_pointer(reader : &PointerReader<'a>, default_value : *Word) -> Self;
}

pub trait FromPointerBuilder<'a> {
    fn init_pointer(PointerBuilder<'a>, uint) -> Self;
    fn get_from_pointer(builder : PointerBuilder<'a>, default_value : *Word) -> Self;
}

pub mod PrimitiveList {
    use super::{FromPointerReader, FromPointerBuilder};
    use layout::{ListReader, ListBuilder, PointerReader, PointerBuilder,
                 PrimitiveElement, POINTER};
    use common::Word;

    pub struct Reader<'a, T> {
        reader : ListReader<'a>
    }

    impl <'a, T : PrimitiveElement> Reader<'a, T> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
            Reader::<'b, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }
    }

    impl <'a, T : PrimitiveElement> FromPointerReader<'a> for Reader<'a, T> {
        fn get_from_pointer(reader : &PointerReader<'a>, default_value : *Word) -> Reader<'a, T> {
            Reader { reader : reader.get_list(POINTER, default_value) }
        }
    }

    impl <'a, T : PrimitiveElement> Index<uint, T> for Reader<'a, T> {
        fn index(&self, index : &uint) -> T {
            PrimitiveElement::get(&self.reader, *index)
        }
    }

    pub struct Builder<'a, T> {
        builder : ListBuilder<'a>
    }

    impl <'a, T : PrimitiveElement> Builder<'a, T> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a, T> {
            Builder { builder : builder }
        }

        pub fn size(&self) -> uint { self.builder.size() }

        pub fn set(&self, index : uint, value : T) {
            PrimitiveElement::set(&self.builder, index, value);
        }
    }

    impl <'a, T : PrimitiveElement> FromPointerBuilder<'a> for Builder<'a, T> {
        fn init_pointer(_builder : PointerBuilder<'a>, _size : uint) -> Builder<'a, T> {
//            builder.init_list(
            fail!();
        }
        fn get_from_pointer(_builder : PointerBuilder<'a>, _default_value : *Word) -> Builder<'a, T> {
            fail!();
        }
    }


    impl <'a, T : PrimitiveElement> Index<uint, T> for Builder<'a, T> {
        fn index(&self, index : &uint) -> T {
            PrimitiveElement::get_from_builder(&self.builder, *index)
        }
    }
}

pub trait ToU16 {
    fn to_u16(self) -> u16;
}


pub mod EnumList {
    use layout::*;
    use list::*;
    use common::Word;

    pub struct Reader<'a, T> {
        reader : ListReader<'a>
    }

    impl <'a, T : FromPrimitive> Reader<'a, T> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
            Reader::<'b, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }

    }

    impl <'a, T : FromPrimitive> FromPointerReader<'a> for Reader<'a, T> {
        fn get_from_pointer(reader : &PointerReader<'a>, default_value : *Word) -> Reader<'a, T> {
            Reader { reader : reader.get_list(TWO_BYTES, default_value) }
        }
    }

    impl <'a, T : FromPrimitive> Index<uint, Option<T>> for Reader<'a, T> {
        fn index(&self, index : &uint) -> Option<T> {
            let result : u16 = PrimitiveElement::get(&self.reader, *index);
            FromPrimitive::from_u16(result)
        }
    }

    pub struct Builder<'a, T> {
        builder : ListBuilder<'a>
    }

    impl <'a, T : ToU16 + FromPrimitive> Builder<'a, T> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a, T> {
            Builder { builder : builder }
        }

        pub fn size(&self) -> uint { self.builder.size() }

        pub fn set(&self, index : uint, value : T) {
            PrimitiveElement::set(&self.builder, index, value.to_u16());
        }
    }

    impl <'a, T : FromPrimitive> FromPointerBuilder<'a> for Builder<'a, T> {
        fn init_pointer(builder : PointerBuilder<'a>, size : uint) -> Builder<'a, T> {
            Builder { builder : builder.init_list(TWO_BYTES, size) }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>, default_value : *Word) -> Builder<'a, T> {
            Builder { builder : builder.get_list(TWO_BYTES, default_value) }
        }
    }


    impl <'a, T : ToU16 + FromPrimitive> Index<uint, Option<T>> for Builder<'a, T> {
        fn index(&self, index : &uint) -> Option<T> {
            let result : u16 = PrimitiveElement::get_from_builder(&self.builder, *index);
            FromPrimitive::from_u16(result)
        }
    }
}

pub mod StructList {
    use super::{FromPointerReader, FromPointerBuilder};
    use common::Word;
    use layout::*;

    pub struct Reader<'a, T> {
        reader : ListReader<'a>
    }

    impl <'a, T : FromStructReader<'a>> Reader<'a, T> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
            Reader::<'b, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }
    }

    impl <'a, T : FromStructReader<'a>> FromPointerReader<'a> for Reader<'a, T> {
        fn get_from_pointer(reader : &PointerReader<'a>, default_value : *Word) -> Reader<'a, T> {
            Reader { reader : reader.get_list(INLINE_COMPOSITE, default_value) }
        }
    }

    impl <'a, T : FromStructReader<'a>> Index<uint, T> for Reader<'a, T> {
        fn index(&self, index : &uint) -> T {
            let result : T = FromStructReader::from_struct_reader(self.reader.get_struct_element(*index));
            result
        }
    }

    pub struct Builder<'a, T> {
        builder : ListBuilder<'a>
    }

    impl <'a, T : FromStructBuilder<'a>> Builder<'a, T> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a, T> {
            Builder { builder : builder }
        }

        pub fn size(&self) -> uint { self.builder.size() }

//        pub fn set(&self, index : uint, value : T) {
//        }
    }

    impl <'a, T : FromStructBuilder<'a> + HasStructSize> FromPointerBuilder<'a> for Builder<'a, T> {
        fn init_pointer(builder : PointerBuilder<'a>, size : uint) -> Builder<'a, T> {
            Builder {
                builder : builder.init_struct_list(size, HasStructSize::struct_size(None::<T>))
            }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>, default_value : *Word) -> Builder<'a, T> {
            Builder {
                builder : builder.get_struct_list(HasStructSize::struct_size(None::<T>), default_value)
            }
        }
    }

    impl <'a, T : FromStructBuilder<'a>> Index<uint, T> for Builder<'a, T> {
        fn index(&self, index : &uint) -> T {
            let result : T =
                FromStructBuilder::from_struct_builder(self.builder.get_struct_element(*index));
            result
        }
    }
}

pub mod ListList {
    use super::{FromPointerReader, FromPointerBuilder};
    use std;
    use common::Word;
    use layout::*;

    pub struct Reader<'a, T> {
        reader : ListReader<'a>
    }

    impl <'a, T> Reader<'a, T> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
            Reader::<'b, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }
    }

    impl <'a, T : FromPointerReader<'a>> FromPointerReader<'a> for Reader<'a, T> {
        fn get_from_pointer(reader : &PointerReader<'a>, default_value : *Word) -> Reader<'a, T> {
            Reader { reader : reader.get_list(POINTER, default_value) }
        }
    }

    impl <'a, T : FromPointerReader<'a>> Index<uint, T> for Reader<'a, T> {
        fn index(&self, index : &uint) -> T {
            assert!(*index <  self.size());
            FromPointerReader::get_from_pointer(
                &self.reader.get_pointer_element(*index), std::ptr::null())
        }
    }

    pub struct Builder<'a, T> {
        builder : ListBuilder<'a>
    }

    impl <'a, T : FromPointerBuilder<'a>> Builder<'a, T> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a, T> {
            Builder { builder : builder }
        }

        pub fn size(&self) -> uint { self.builder.size() }
    }


    impl <'a, T : FromPointerBuilder<'a>> FromPointerBuilder<'a> for Builder<'a, T> {
        fn init_pointer(builder : PointerBuilder<'a>, size : uint) -> Builder<'a, T> {
            Builder {
                builder : builder.init_list(POINTER, size)
            }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>, default_value : *Word) -> Builder<'a, T> {
            Builder {
                builder : builder.get_list(POINTER, default_value)
            }
        }
    }

    impl <'a, T : FromPointerBuilder<'a>> Index<uint, T> for Builder<'a, T> {
        fn index(&self, index : &uint) -> T {
            let result : T =
                FromPointerBuilder::get_from_pointer(
                self.builder.get_pointer_element(*index),
                std::ptr::null());
            result
        }
    }

}

// TODO BlobList
