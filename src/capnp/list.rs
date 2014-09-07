/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use layout::{PointerBuilder, PointerReader};
use common::Word;


// TODO maybe we can simplify these traits. It seems that the only
// difference in the implementations is the FieldSize.

pub trait FromPointerReader<'a> {
    fn get_from_pointer(reader : &PointerReader<'a>, default_value : *const Word) -> Self;
}

pub trait FromPointerBuilder<'a> {
    fn init_pointer(PointerBuilder<'a>, uint) -> Self;
    fn get_from_pointer(builder : PointerBuilder<'a>, default_value : *const Word) -> Self;
}

pub trait IndexMove<I,T> {
    fn index_move(&self, index : I) -> T;
}

pub struct ListIter<T> {
    list : T,
    index : uint,
    size : uint,
}

impl <T> ListIter<T> {
    pub fn new(list : T, size : uint) -> ListIter<T> {
        ListIter { list : list, index : 0, size : size }
    }
}


impl <U, T : IndexMove<uint, U>> ::std::iter::Iterator<U> for ListIter<T> {
    fn next(&mut self) -> ::std::option::Option<U> {
        if self.index < self.size {
            let result = self.list.index_move(self.index);
            self.index += 1;
            return Some(result);
        } else {
            return None;
        }
    }
}

pub mod primitive_list {
    use super::{FromPointerReader, FromPointerBuilder};
    use layout::{ListReader, ListBuilder, PointerReader, PointerBuilder,
                 PrimitiveElement, element_size_for_type};
    use common::Word;

    pub struct Reader<'a, T> {
        // I want this field to be private, but then I can't access it in set_list()
        pub reader : ListReader<'a>
    }

    impl <'a, T : PrimitiveElement> Reader<'a, T> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
            Reader::<'b, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }
    }

    impl <'a, T : PrimitiveElement> FromPointerReader<'a> for Reader<'a, T> {
        fn get_from_pointer(reader : &PointerReader<'a>, default_value : *const Word) -> Reader<'a, T> {
            Reader { reader : reader.get_list(element_size_for_type::<T>(), default_value) }
        }
    }

    impl <'a, T : PrimitiveElement> Reader<'a, T> {
        pub fn get(&self, index : uint) -> T {
            assert!(index < self.size());
            PrimitiveElement::get(&self.reader, index)
        }
    }

    pub struct Builder<'a, T> {
        pub builder : ListBuilder<'a>
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
        fn init_pointer(builder : PointerBuilder<'a>, size : uint) -> Builder<'a, T> {
            Builder { builder : builder.init_list(element_size_for_type::<T>(), size) }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>, default_value : *const Word) -> Builder<'a, T> {
            Builder { builder : builder.get_list(element_size_for_type::<T>(), default_value) }
        }
    }

    impl <'a, T : PrimitiveElement> Builder<'a, T> {
        pub fn get(&self, index : uint) -> T {
            assert!(index < self.size());
            PrimitiveElement::get_from_builder(&self.builder, index)
        }
    }
}

pub trait ToU16 {
    fn to_u16(self) -> u16;
}


pub mod enum_list {
    use super::{FromPointerReader, FromPointerBuilder, ToU16};
    use layout::{ListReader, ListBuilder, PointerReader, PointerBuilder,
                 TwoBytes, PrimitiveElement};
    use common::Word;

    pub struct Reader<'a, T> {
        pub reader : ListReader<'a>
    }

    impl <'a, T : FromPrimitive> Reader<'a, T> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
            Reader::<'b, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }

    }

    impl <'a, T : FromPrimitive> FromPointerReader<'a> for Reader<'a, T> {
        fn get_from_pointer(reader : &PointerReader<'a>, default_value : *const Word) -> Reader<'a, T> {
            Reader { reader : reader.get_list(TwoBytes, default_value) }
        }
    }

    impl <'a, T : FromPrimitive> Reader<'a, T> {
        pub fn get(&self, index : uint) -> Option<T> {
            assert!(index < self.size());
            let result : u16 = PrimitiveElement::get(&self.reader, index);
            FromPrimitive::from_u16(result)
        }
    }

    pub struct Builder<'a, T> {
        pub builder : ListBuilder<'a>
    }

    impl <'a, T : ToU16 + FromPrimitive> Builder<'a, T> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a, T> {
            Builder { builder : builder }
        }

        pub fn size(&self) -> uint { self.builder.size() }

        pub fn set(&self, index : uint, value : T) {
            assert!(index < self.size());
            PrimitiveElement::set(&self.builder, index, value.to_u16());
        }
    }

    impl <'a, T : FromPrimitive> FromPointerBuilder<'a> for Builder<'a, T> {
        fn init_pointer(builder : PointerBuilder<'a>, size : uint) -> Builder<'a, T> {
            Builder { builder : builder.init_list(TwoBytes, size) }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>, default_value : *const Word) -> Builder<'a, T> {
            Builder { builder : builder.get_list(TwoBytes, default_value) }
        }
    }

    impl <'a, T : ToU16 + FromPrimitive>  Builder<'a, T> {
        pub fn get(&self, index : uint) -> Option<T> {
            assert!(index < self.size());
            let result : u16 = PrimitiveElement::get_from_builder(&self.builder, index);
            FromPrimitive::from_u16(result)
        }
    }
}


pub mod struct_list {
    use super::{FromPointerReader, FromPointerBuilder};
    use common::Word;
    use layout::{ListReader, ListBuilder, PointerReader, PointerBuilder,
                 InlineComposite, FromStructBuilder, FromStructReader,
                 HasStructSize};


    pub struct Reader<'a, T> {
        pub reader : ListReader<'a>
    }

    impl <'a, T : FromStructReader<'a>> Reader<'a, T> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
            Reader::<'b, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }

        pub fn iter(self) -> super::ListIter<Reader<'a, T>> {
            return super::ListIter::new(self, self.size());
        }
    }

    impl <'a, T : FromStructReader<'a>> FromPointerReader<'a> for Reader<'a, T> {
        fn get_from_pointer(reader : &PointerReader<'a>, default_value : *const Word) -> Reader<'a, T> {
            Reader { reader : reader.get_list(InlineComposite, default_value) }
        }
    }

    impl <'a, T : FromStructReader<'a>>  super::IndexMove<uint, T> for Reader<'a, T> {
        fn index_move(&self, index : uint) -> T {
            self.get(index)
        }
    }

    impl <'a, T : FromStructReader<'a>> Reader<'a, T> {
        pub fn get(&self, index : uint) -> T {
            assert!(index < self.size());
            let result : T = FromStructReader::new(self.reader.get_struct_element(index));
            result
        }
    }

    pub struct Builder<'a, T> {
        pub builder : ListBuilder<'a>
    }

    impl <'a, T : FromStructBuilder<'a>> Builder<'a, T> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a, T> {
            Builder { builder : builder }
        }

        pub fn size(&self) -> uint { self.builder.size() }

//        pub fn set(&self, index : uint, value : T) {
//        }

        pub fn iter(self) -> super::ListIter<Builder<'a, T>> {
            return super::ListIter::new(self, self.size());
        }

    }

    impl <'a, T : FromStructBuilder<'a> + HasStructSize> FromPointerBuilder<'a> for Builder<'a, T> {
        fn init_pointer(builder : PointerBuilder<'a>, size : uint) -> Builder<'a, T> {
            Builder {
                builder : builder.init_struct_list(size, HasStructSize::struct_size(None::<T>))
            }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>, default_value : *const Word) -> Builder<'a, T> {
            Builder {
                builder : builder.get_struct_list(HasStructSize::struct_size(None::<T>), default_value)
            }
        }
    }

    impl <'a, T : FromStructBuilder<'a>> super::IndexMove<uint, T> for Builder<'a, T> {
        fn index_move(&self, index : uint) -> T {
            self.get(index)
        }
    }

    impl <'a, T : FromStructBuilder<'a>> Builder<'a, T> {
        pub fn get(&self, index : uint) -> T {
            assert!(index < self.size());
            let result : T =
                FromStructBuilder::new(self.builder.get_struct_element(index));
            result

        }
    }

}

pub mod list_list {
    use super::{FromPointerReader, FromPointerBuilder};
    use common::Word;
    use layout::{ListReader, ListBuilder, PointerReader, PointerBuilder, Pointer};

    pub struct Reader<'a, T> {
        pub reader : ListReader<'a>
    }

    impl <'a, T> Reader<'a, T> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
            Reader::<'b, T> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }
    }

    impl <'a, T : FromPointerReader<'a>> FromPointerReader<'a> for Reader<'a, T> {
        fn get_from_pointer(reader : &PointerReader<'a>, default_value : *const Word) -> Reader<'a, T> {
            Reader { reader : reader.get_list(Pointer, default_value) }
        }
    }

    impl <'a, T : FromPointerReader<'a>> Reader<'a, T> {
        pub fn get(&self, index : uint) -> T {
            assert!(index <  self.size());
            FromPointerReader::get_from_pointer(
                &self.reader.get_pointer_element(index), ::std::ptr::null())
        }
    }

    pub struct Builder<'a, T> {
        pub builder : ListBuilder<'a>
    }

    impl <'a, T : FromPointerBuilder<'a>> Builder<'a, T> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a, T> {
            Builder { builder : builder }
        }

        pub fn size(&self) -> uint { self.builder.size() }

        pub fn init(&self, index : uint, size : uint) -> T {
            let result : T =
                FromPointerBuilder::init_pointer(self.builder.get_pointer_element(index), size);
            result
        }
    }


    impl <'a, T : FromPointerBuilder<'a>> FromPointerBuilder<'a> for Builder<'a, T> {
        fn init_pointer(builder : PointerBuilder<'a>, size : uint) -> Builder<'a, T> {
            Builder {
                builder : builder.init_list(Pointer, size)
            }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>, default_value : *const Word) -> Builder<'a, T> {
            Builder {
                builder : builder.get_list(Pointer, default_value)
            }
        }
    }

    impl <'a, T : FromPointerBuilder<'a>> Builder<'a, T> {
        pub fn get(&self, index : uint) -> T {
            assert!(index < self.size());
            let result : T =
                FromPointerBuilder::get_from_pointer(
                self.builder.get_pointer_element(index),
                ::std::ptr::null());
            result
        }
    }

}

pub mod text_list {
    use super::{FromPointerReader, FromPointerBuilder};
    use common::Word;
    use blob::text;
    use layout::*;

    pub struct Reader<'a> {
        pub reader : ListReader<'a>
    }

    impl <'a> Reader<'a> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b> {
            Reader::<'b> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }
    }

    impl <'a> FromPointerReader<'a> for Reader<'a> {
        fn get_from_pointer(reader : &PointerReader<'a>, default_value : *const Word) -> Reader<'a> {
            Reader { reader : reader.get_list(Pointer, default_value) }
        }
    }

    impl <'a> Reader<'a> {
        pub fn get(&self, index : uint) -> text::Reader<'a> {
            assert!(index <  self.size());
            self.reader.get_pointer_element(index).get_text(::std::ptr::null(), 0)
        }
    }

    pub struct Builder<'a> {
        pub builder : ListBuilder<'a>
    }

    impl <'a> Builder<'a> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a> {
            Builder { builder : builder }
        }

        pub fn size(&self) -> uint { self.builder.size() }

        pub fn set(&self, index : uint, value : text::Reader) {
            assert!(index < self.size());
            self.builder.get_pointer_element(index).set_text(value);
        }
    }


    impl <'a> FromPointerBuilder<'a> for Builder<'a> {
        fn init_pointer(builder : PointerBuilder<'a>, size : uint) -> Builder<'a> {
            Builder {
                builder : builder.init_list(Pointer, size)
            }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>, default_value : *const Word) -> Builder<'a> {
            Builder {
                builder : builder.get_list(Pointer, default_value)
            }
        }
    }

    impl <'a> Builder<'a> {
        pub fn get(&self, index : uint) -> text::Builder<'a> {
            self.builder.get_pointer_element(index).get_text(::std::ptr::null(), 0)
        }
    }

}

pub mod data_list {
    use super::{FromPointerReader, FromPointerBuilder};
    use common::Word;
    use blob::data;
    use layout::*;

    pub struct Reader<'a> {
        pub reader : ListReader<'a>
    }

    impl <'a> Reader<'a> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b> {
            Reader::<'b> { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }
    }

    impl <'a> FromPointerReader<'a> for Reader<'a> {
        fn get_from_pointer(reader : &PointerReader<'a>, default_value : *const Word) -> Reader<'a> {
            Reader { reader : reader.get_list(Pointer, default_value) }
        }
    }

    impl <'a> Reader<'a> {
        pub fn get(&self, index : uint) -> data::Reader<'a> {
            assert!(index <  self.size());
            self.reader.get_pointer_element(index).get_data(::std::ptr::null(), 0)
        }
    }

    pub struct Builder<'a> {
        pub builder : ListBuilder<'a>
    }

    impl <'a> Builder<'a> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a> {
            Builder { builder : builder }
        }

        pub fn size(&self) -> uint { self.builder.size() }

        pub fn set(&self, index : uint, value : data::Reader) {
            assert!(index < self.size());
            self.builder.get_pointer_element(index).set_data(value);
        }
    }


    impl <'a> FromPointerBuilder<'a> for Builder<'a> {
        fn init_pointer(builder : PointerBuilder<'a>, size : uint) -> Builder<'a> {
            Builder {
                builder : builder.init_list(Pointer, size)
            }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>, default_value : *const Word) -> Builder<'a> {
            Builder {
                builder : builder.get_list(Pointer, default_value)
            }
        }
    }

    impl <'a> Builder<'a> {
        pub fn get(&self, index : uint) -> data::Builder<'a> {
            assert!(index < self.size());
            self.builder.get_pointer_element(index).get_data(::std::ptr::null(), 0)
        }
    }

}
