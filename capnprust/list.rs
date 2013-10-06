/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

pub mod PrimitiveList {
    use layout::*;

    pub struct Reader<'self, T> {
        reader : ListReader<'self>
    }

    impl <'self, T : PrimitiveElement> Reader<'self, T> {
        pub fn new<'a>(reader : ListReader<'a>) -> Reader<'a, T> {
            Reader::<'a, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }

        pub fn get(&self, index : uint) -> T {
            PrimitiveElement::get(&self.reader, index)
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

        pub fn get(&self, index : uint) -> T {
            PrimitiveElement::getFromBuilder(&self.builder, index)
        }

        pub fn set(&self, index : uint, value : T) {
            PrimitiveElement::set(&self.builder, index, value);
        }

    }
}

pub trait ToU16 {
    fn to_u16(self) -> u16;
}


pub mod EnumList {
    use layout::*;
    use list::*;

    pub struct Reader<'self, T> {
        reader : ListReader<'self>
    }

    impl <'self, T : FromPrimitive> Reader<'self, T> {
        pub fn new<'a>(reader : ListReader<'a>) -> Reader<'a, T> {
            Reader::<'a, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }

        pub fn get(&self, index : uint) -> Option<T> {
            let result : u16 = PrimitiveElement::get(&self.reader, index);
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

        pub fn get(&self, index : uint) -> Option<T> {
            let result : u16 = PrimitiveElement::getFromBuilder(&self.builder, index);
            FromPrimitive::from_u16(result)
        }

        pub fn set(&self, index : uint, value : T) {
            PrimitiveElement::set(&self.builder, index, value.to_u16());
        }
    }
}

// The struct list reader needs to be able to instantiate element readers
// of the appropriate type. It is implemented as a macro.
