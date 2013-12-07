/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

pub mod AnyPointer {
    use std;
    use layout::{PointerReader, PointerBuilder, FromStructReader, FromStructBuilder,
                 HasStructSize};

    pub struct Reader<'a> {
        reader : PointerReader<'a>
    }

    impl <'a> Reader<'a> {
        #[inline]
        pub fn new<'b>(reader : PointerReader<'b>) -> Reader<'b> {
            Reader { reader : reader }
        }

        #[inline]
        pub fn is_null(&self) -> bool {
            self.reader.is_null()
        }

        #[inline]
        pub fn get_as_struct<T : FromStructReader<'a>>(&self) -> T {
            FromStructReader::from_struct_reader(self.reader.get_struct(std::ptr::null()))
        }
    }

    pub struct Builder {
        builder : PointerBuilder
    }

    impl Builder {
        #[inline]
        pub fn new<'b>(builder : PointerBuilder) -> Builder {
            Builder { builder : builder }
        }

        pub fn init_as_struct<T : FromStructBuilder + HasStructSize>(&self) -> T {
            FromStructBuilder::from_struct_builder(
                self.builder.init_struct(
                    HasStructSize::struct_size(None::<T>)))
        }
    }
}
