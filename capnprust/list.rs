/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

pub mod PrimitiveList {
    use layout::{ListReader, ListBuilder, PrimitiveElement};

    pub struct Reader<'a, T> {
        reader : ListReader<'a>
    }

    impl <'self, T : PrimitiveElement> Reader<'self, T> {
        pub fn new<'a>(reader : ListReader<'a>) -> Reader<'a, T> {
            Reader::<'a, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }
    }

    impl <'a, T : PrimitiveElement> Index<uint, T> for Reader<'a, T> {
        fn index(&self, index : &uint) -> T {
            PrimitiveElement::get(&self.reader, *index)
        }
    }

    pub struct Builder<T> {
        builder : ListBuilder
    }

    impl <T : PrimitiveElement> Builder<T> {
        pub fn new(builder : ListBuilder) -> Builder<T> {
            Builder { builder : builder }
        }

        pub fn size(&self) -> uint { self.builder.size() }

        pub fn set(&self, index : uint, value : T) {
            PrimitiveElement::set(&self.builder, index, value);
        }
    }

    impl <T : PrimitiveElement> Index<uint, T> for Builder<T> {
        fn index(&self, index : &uint) -> T {
            PrimitiveElement::getFromBuilder(&self.builder, *index)
        }
    }
}

pub trait ToU16 {
    fn to_u16(self) -> u16;
}


pub mod EnumList {
    use layout::*;
    use list::*;

    pub struct Reader<'a, T> {
        reader : ListReader<'a>
    }

    impl <'a, T : FromPrimitive> Reader<'a, T> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
            Reader::<'b, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }

    }

    impl <'a, T : FromPrimitive> Index<uint, Option<T>> for Reader<'a, T> {
        fn index(&self, index : &uint) -> Option<T> {
            let result : u16 = PrimitiveElement::get(&self.reader, *index);
            FromPrimitive::from_u16(result)
        }
    }

    pub struct Builder<T> {
        builder : ListBuilder
    }

    impl <T : ToU16 + FromPrimitive> Builder<T> {
        pub fn new(builder : ListBuilder) -> Builder<T> {
            Builder { builder : builder }
        }

        pub fn size(&self) -> uint { self.builder.size() }

        pub fn set(&self, index : uint, value : T) {
            PrimitiveElement::set(&self.builder, index, value.to_u16());
        }
    }

    impl <T : ToU16 + FromPrimitive> Index<uint, Option<T>> for Builder<T> {
        fn index(&self, index : &uint) -> Option<T> {
            let result : u16 = PrimitiveElement::getFromBuilder(&self.builder, *index);
            FromPrimitive::from_u16(result)
        }
    }
}

// The struct list reader needs to be able to instantiate element readers
// of the appropriate type. It is implemented as a macro.
