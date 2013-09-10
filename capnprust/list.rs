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

pub trait HasMaxEnumerant {
    fn maxEnumerant(_unused_self : Option<Self>) -> u16;
    fn cast(unused_self : Option<Self>, value : u16) -> Option<Self> {
        use std;
        if (value > HasMaxEnumerant::maxEnumerant(unused_self)) { None }
        else {Some (unsafe {std::cast::transmute(value as uint)})}
    }

    // Do I really have to define a method for this?
    fn asU16(self) -> u16;
}


pub mod EnumList {
    use layout::*;
    use list::*;

    pub struct Reader<'self, T> {
        reader : ListReader<'self>
    }

    impl <'self, T : HasMaxEnumerant> Reader<'self, T> {
        pub fn new<'a>(reader : ListReader<'a>) -> Reader<'a, T> {
            Reader::<'a, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }

        pub fn get(&self, index : uint) -> Option<T> {
            let result : u16 = PrimitiveElement::get(&self.reader, index);
            let unused_self : Option<T> = None;
            HasMaxEnumerant::cast(unused_self, result)
        }
    }

    pub struct Builder<T> {
        builder : ListBuilder
    }

    impl <T : HasMaxEnumerant> Builder<T> {
        pub fn new(builder : ListBuilder) -> Builder<T> {
            Builder { builder : builder }
        }

        pub fn size(&self) -> uint { self.builder.size() }

        pub fn get(&self, index : uint) -> Option<T> {
            let result : u16 = PrimitiveElement::getFromBuilder(&self.builder, index);
            let unused_self : Option<T> = None;
            HasMaxEnumerant::cast(unused_self, result)
        }

        pub fn set(&self, index : uint, value : T) {
            PrimitiveElement::set(&self.builder, index, value.asU16());
        }
    }
}

// The struct list reader needs to be able to instantiate element readers
// of the appropriate type. It is implemented as a macro.
