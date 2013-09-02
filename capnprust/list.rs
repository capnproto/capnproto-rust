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

    impl <'self, T : Clone> Reader<'self, T> {
        pub fn new<'a>(reader : ListReader<'a>) -> Reader<'a, T> {
            Reader::<'a, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }

        pub fn get(&self, index : uint) -> T {
            self.reader.getDataElement(index)
        }
    }

    pub struct Builder<T> {
        builder : ListBuilder
    }

    impl <T : Clone> Builder<T> {
        pub fn new(builder : ListBuilder) -> Builder<T> {
            Builder { builder : builder }
        }

        pub fn size(&self) -> uint { self.builder.size() }

        pub fn get(&self, index : uint) -> T {
            self.builder.getDataElement(index)
        }

        pub fn set(&self, index : uint, value : T) {
            self.builder.setDataElement(index, value)
        }

    }
}

// The struct list reader needs to be able to instantiate element readers
// of the appropriate type. It is implemented as a macro.
