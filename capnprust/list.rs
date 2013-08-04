/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

pub mod PrimitiveList {
    use layout::*;

    pub struct Reader<'self> {
        reader : ListReader<'self>
    }

    impl <'self> Reader<'self> {
        pub fn new<'a>(reader : ListReader<'a>) -> Reader<'a> {
            Reader { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }

        pub fn get<T : Clone>(&self, index : uint) -> T {
            self.reader.getDataElement(index)
        }
    }

    pub struct Builder {
        builder : ListBuilder
    }

    impl Builder {
        pub fn new(builder : ListBuilder) -> Builder {
            Builder { builder : builder }
        }

        // TODO
    }
}

// The struct list reader needs to be able to instantiate element readers
// of the appropriate type. It is implemented as a macro.
