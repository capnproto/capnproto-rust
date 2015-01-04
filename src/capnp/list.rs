/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

trait IndexMove<I, T> {
    fn index_move(&self, index : I) -> T;
}

pub struct ListIter<T, U> {
    list : T,
    index : u32,
    size : u32,
}

impl <T, U> ListIter<T, U> {
    pub fn new(list : T, size : u32) -> ListIter<T, U> {
        ListIter { list : list, index : 0, size : size }
    }
}

impl <U, T : IndexMove<u32, U>> ::std::iter::Iterator for ListIter<T, U> {
    type Item = U;
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
    use traits::{FromPointerReader, FromPointerBuilder};
    use layout::{ListReader, ListBuilder, PointerReader, PointerBuilder,
                 PrimitiveElement, element_size_for_type};

    #[derive(Copy)]
    pub struct Reader<'a, T> {
        reader : ListReader<'a>
    }

    impl <'a, T : PrimitiveElement> Reader<'a, T> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
            Reader::<'b, T> { reader : reader }
        }

        pub fn len(&self) -> u32 { self.reader.len() }
    }

    impl <'a, T : PrimitiveElement> FromPointerReader<'a> for Reader<'a, T> {
        fn get_from_pointer(reader : &PointerReader<'a>) -> Reader<'a, T> {
            Reader { reader : reader.get_list(element_size_for_type::<T>(), ::std::ptr::null()) }
        }
    }

    impl <'a, T : PrimitiveElement> Reader<'a, T> {
        pub fn get(&self, index : u32) -> T {
            assert!(index < self.len());
            PrimitiveElement::get(&self.reader, index)
        }
    }

    pub struct Builder<'a, T> {
        builder : ListBuilder<'a>
    }

    impl <'a, T : PrimitiveElement> Builder<'a, T> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a, T> {
            Builder { builder : builder }
        }

        pub fn len(&self) -> u32 { self.builder.len() }

        pub fn set(&mut self, index : u32, value : T) {
            PrimitiveElement::set(&self.builder, index, value);
        }
    }

    impl <'a, T : PrimitiveElement> FromPointerBuilder<'a> for Builder<'a, T> {
        fn init_pointer(builder : PointerBuilder<'a>, size : u32) -> Builder<'a, T> {
            Builder { builder : builder.init_list(element_size_for_type::<T>(), size) }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>) -> Builder<'a, T> {
            Builder { builder : builder.get_list(element_size_for_type::<T>(), ::std::ptr::null())}
        }
    }

    impl <'a, T : PrimitiveElement> Builder<'a, T> {
        pub fn get(&self, index : u32) -> T {
            assert!(index < self.len());
            PrimitiveElement::get_from_builder(&self.builder, index)
        }
    }

    impl <'a, T> ::traits::SetPointerBuilder<Builder<'a, T>> for Reader<'a, T> {
        fn set_pointer_builder<'b>(pointer : ::layout::PointerBuilder<'b>, value : Reader<'a, T>) {
            pointer.set_list(&value.reader);
        }
    }

    impl <'a, 'b : 'a, T> ::traits::CastableTo<Builder<'a, T> > for Builder<'b, T> {
        fn cast(self) -> Builder<'a, T> {
            Builder { builder : self.builder }
        }
    }
}

pub mod enum_list {
    use traits::{FromPointerReader, FromPointerBuilder, ToU16};
    use layout::{ListReader, ListBuilder, PointerReader, PointerBuilder,
                 TwoBytes, PrimitiveElement};

    #[derive(Copy)]
    pub struct Reader<'a, T> {
        reader : ListReader<'a>
    }

    impl <'a, T : ::std::num::FromPrimitive> Reader<'a, T> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
            Reader::<'b, T> { reader : reader }
        }

        pub fn len(&self) -> u32 { self.reader.len() }

    }

    impl <'a, T : ::std::num::FromPrimitive> FromPointerReader<'a> for Reader<'a, T> {
        fn get_from_pointer(reader : &PointerReader<'a>) -> Reader<'a, T> {
            Reader { reader : reader.get_list(TwoBytes, ::std::ptr::null()) }
        }
    }

    impl <'a, T : ::std::num::FromPrimitive> Reader<'a, T> {
        pub fn get(&self, index : u32) -> Option<T> {
            assert!(index < self.len());
            let result : u16 = PrimitiveElement::get(&self.reader, index);
            ::std::num::FromPrimitive::from_u16(result)
        }
    }

    pub struct Builder<'a, T> {
        builder : ListBuilder<'a>
    }

    impl <'a, T : ToU16 + ::std::num::FromPrimitive> Builder<'a, T> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a, T> {
            Builder { builder : builder }
        }

        pub fn len(&self) -> u32 { self.builder.len() }

        pub fn set(&mut self, index : u32, value : T) {
            assert!(index < self.len());
            PrimitiveElement::set(&self.builder, index, value.to_u16());
        }
    }

    impl <'a, T : ::std::num::FromPrimitive> FromPointerBuilder<'a> for Builder<'a, T> {
        fn init_pointer(builder : PointerBuilder<'a>, size : u32) -> Builder<'a, T> {
            Builder { builder : builder.init_list(TwoBytes, size) }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>) -> Builder<'a, T> {
            Builder { builder : builder.get_list(TwoBytes, ::std::ptr::null()) }
        }
    }

    impl <'a, T : ToU16 + ::std::num::FromPrimitive>  Builder<'a, T> {
        pub fn get(&self, index : u32) -> Option<T> {
            assert!(index < self.len());
            let result : u16 = PrimitiveElement::get_from_builder(&self.builder, index);
            ::std::num::FromPrimitive::from_u16(result)
        }
    }

    impl <'a, T> ::traits::SetPointerBuilder<Builder<'a, T>> for Reader<'a, T> {
        fn set_pointer_builder<'b>(pointer : ::layout::PointerBuilder<'b>, value : Reader<'a, T>) {
            pointer.set_list(&value.reader);
        }
    }

    impl <'a, 'b : 'a, T> ::traits::CastableTo<Builder<'a, T> > for Builder<'b, T> {
        fn cast(self) -> Builder<'a, T> {
            Builder { builder : self.builder }
        }
    }
}


pub mod struct_list {
    use layout::{ListReader, ListBuilder, PointerReader, PointerBuilder, InlineComposite};
    use traits::{FromPointerReader, FromPointerBuilder,
                 FromStructBuilder, FromStructReader, HasStructSize};

    pub struct Reader<'a, T> {
        reader : ListReader<'a>
    }

    impl <'a, T> Copy for Reader<'a, T> {}

    impl <'a, T : FromStructReader<'a>> Reader<'a, T> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
            Reader::<'b, T> { reader : reader }
        }

        pub fn len(&self) -> u32 { self.reader.len() }

        pub fn iter(self) -> super::ListIter<Reader<'a, T>, T> {
            return super::ListIter::new(self, self.len());
        }
    }

    impl <'a, T, U> Reader<'a, T> where T : ::traits::CastableTo<U> {
        pub fn borrow<'b>(&'b self) -> Reader<'b, U> {
            Reader {reader : self.reader}
        }
    }


    impl <'a, T : FromStructReader<'a>> FromPointerReader<'a> for Reader<'a, T> {
        fn get_from_pointer(reader : &PointerReader<'a>) -> Reader<'a, T> {
            Reader { reader : reader.get_list(InlineComposite, ::std::ptr::null()) }
        }
    }

    impl <'a, T : FromStructReader<'a>>  super::IndexMove<u32, T> for Reader<'a, T> {
        fn index_move(&self, index : u32) -> T {
            self.get(index)
        }
    }

    impl <'a, T : FromStructReader<'a>> Reader<'a, T> {
        pub fn get(self, index : u32) -> T {
            assert!(index < self.len());
            let result : T = FromStructReader::new(self.reader.get_struct_element(index));
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

        pub fn len(&self) -> u32 { self.builder.len() }

//        pub fn set(&self, index : uint, value : T) {
//        }

    }

    impl <'a, T, U> Builder<'a, T> where T : ::traits::CastableTo<U> {
        pub fn borrow<'b>(&'b mut self) -> Builder<'b, U> {
            Builder {builder : self.builder}
        }
    }

    impl <'a, T : FromStructBuilder<'a> + HasStructSize> FromPointerBuilder<'a> for Builder<'a, T> {
        fn init_pointer(builder : PointerBuilder<'a>, size : u32) -> Builder<'a, T> {
            Builder {
                builder : builder.init_struct_list(size, HasStructSize::struct_size(None::<T>))
            }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>) -> Builder<'a, T> {
            Builder {
                builder : builder.get_struct_list(HasStructSize::struct_size(None::<T>), ::std::ptr::null())
            }
        }
    }

    impl <'a, T : FromStructBuilder<'a>> Builder<'a, T> {
        pub fn get(self, index : u32) -> T {
            assert!(index < self.len());
            let result : T =
                FromStructBuilder::new(self.builder.get_struct_element(index));
            result

        }
    }

    impl <'a, T> ::traits::SetPointerBuilder<Builder<'a, T>> for Reader<'a, T> {
        fn set_pointer_builder<'b>(pointer : ::layout::PointerBuilder<'b>, value : Reader<'a, T>) {
            pointer.set_list(&value.reader);
        }
    }

    impl <'a, 'b : 'a, T, U : ::traits::CastableTo<T>> ::traits::CastableTo<Builder<'a, T> > for Builder<'b, U> {
        fn cast(self) -> Builder<'a, T> {
            Builder { builder : self.builder }
        }
    }
}

pub mod list_list {
    use traits::{FromPointerReader, FromPointerBuilder};
    use layout::{ListReader, ListBuilder, PointerReader, PointerBuilder, Pointer};

    #[derive(Copy)]
    pub struct Reader<'a, T> {
        reader : ListReader<'a>
    }

    impl <'a, T> Reader<'a, T> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
            Reader::<'b, T> { reader : reader }
        }

        pub fn len(&self) -> u32 { self.reader.len() }
    }

    impl <'a, T : FromPointerReader<'a>> FromPointerReader<'a> for Reader<'a, T> {
        fn get_from_pointer(reader : &PointerReader<'a>) -> Reader<'a, T> {
            Reader { reader : reader.get_list(Pointer, ::std::ptr::null()) }
        }
    }

    impl <'a, T : FromPointerReader<'a>> Reader<'a, T> {
        pub fn get(self, index : u32) -> T {
            assert!(index <  self.len());
            FromPointerReader::get_from_pointer(&self.reader.get_pointer_element(index))
        }
    }

    pub struct Builder<'a, T> {
        builder : ListBuilder<'a>
    }

    impl <'a, T : FromPointerBuilder<'a>> Builder<'a, T> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a, T> {
            Builder { builder : builder }
        }

        pub fn len(&self) -> u32 { self.builder.len() }

        pub fn init(self, index : u32, size : u32) -> T {
            let result : T =
                FromPointerBuilder::init_pointer(self.builder.get_pointer_element(index), size);
            result
        }
    }

    impl <'a, T, U> Builder<'a, T> where T : ::traits::CastableTo<U> {
        pub fn borrow<'b>(&'b mut self) -> Builder<'b, U> {
            Builder {builder : self.builder}
        }
    }

    impl <'a, T : FromPointerBuilder<'a>> FromPointerBuilder<'a> for Builder<'a, T> {
        fn init_pointer(builder : PointerBuilder<'a>, size : u32) -> Builder<'a, T> {
            Builder {
                builder : builder.init_list(Pointer, size)
            }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>) -> Builder<'a, T> {
            Builder {
                builder : builder.get_list(Pointer, ::std::ptr::null())
            }
        }
    }

    impl <'a, T : FromPointerBuilder<'a>> Builder<'a, T> {
        pub fn get(self, index : u32) -> T {
            assert!(index < self.len());
            FromPointerBuilder::get_from_pointer(self.builder.get_pointer_element(index))
        }
    }

    impl <'a, T> ::traits::SetPointerBuilder<Builder<'a, T>> for Reader<'a, T> {
        fn set_pointer_builder<'b>(pointer : ::layout::PointerBuilder<'b>, value : Reader<'a, T>) {
            pointer.set_list(&value.reader);
        }
    }

    impl <'a, 'b : 'a, T, U : ::traits::CastableTo<T>> ::traits::CastableTo<Builder<'a, T> > for Builder<'b, U> {
        fn cast(self) -> Builder<'a, T> {
            Builder { builder : self.builder }
        }
    }
}

pub mod text_list {
    use traits::{FromPointerReader, FromPointerBuilder};
    use blob::text;
    use layout::{ListBuilder, ListReader, Pointer, PointerBuilder, PointerReader};

    #[derive(Copy)]
    pub struct Reader<'a> {
        reader : ListReader<'a>
    }

    impl <'a> Reader<'a> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b> {
            Reader::<'b> { reader : reader }
        }

        pub fn len(&self) -> u32 { self.reader.len() }
    }

    impl <'a> FromPointerReader<'a> for Reader<'a> {
        fn get_from_pointer(reader : &PointerReader<'a>) -> Reader<'a> {
            Reader { reader : reader.get_list(Pointer, ::std::ptr::null()) }
        }
    }

    impl <'a> Reader<'a> {
        pub fn get(self, index : u32) -> text::Reader<'a> {
            assert!(index <  self.len());
            self.reader.get_pointer_element(index).get_text(::std::ptr::null(), 0)
        }
    }

    pub struct Builder<'a> {
        builder : ListBuilder<'a>
    }

    impl <'a> Builder<'a> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a> {
            Builder { builder : builder }
        }

        pub fn len(&self) -> u32 { self.builder.len() }

        pub fn set(&mut self, index : u32, value : text::Reader) {
            assert!(index < self.len());
            self.builder.get_pointer_element(index).set_text(value);
        }

        pub fn borrow<'b>(&'b mut self) -> Builder<'b> {
            Builder {builder : self.builder}
        }
    }


    impl <'a> FromPointerBuilder<'a> for Builder<'a> {
        fn init_pointer(builder : PointerBuilder<'a>, size : u32) -> Builder<'a> {
            Builder {
                builder : builder.init_list(Pointer, size)
            }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>) -> Builder<'a> {
            Builder {
                builder : builder.get_list(Pointer, ::std::ptr::null())
            }
        }
    }

    impl <'a> Builder<'a> {
        pub fn get(self, index : u32) -> text::Builder<'a> {
            self.builder.get_pointer_element(index).get_text(::std::ptr::null(), 0)
        }
    }

    impl <'a> ::traits::SetPointerBuilder<Builder<'a>> for Reader<'a> {
        fn set_pointer_builder<'b>(pointer : ::layout::PointerBuilder<'b>, value : Reader<'a>) {
            pointer.set_list(&value.reader);
        }
    }
}

pub mod data_list {
    use traits::{FromPointerReader, FromPointerBuilder};
    use blob::data;
    use layout::*;

    #[derive(Copy)]
    pub struct Reader<'a> {
        pub reader : ListReader<'a>
    }

    impl <'a> Reader<'a> {
        pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b> {
            Reader::<'b> { reader : reader }
        }

        pub fn len(&self) -> u32 { self.reader.len() }
    }

    impl <'a> FromPointerReader<'a> for Reader<'a> {
        fn get_from_pointer(reader : &PointerReader<'a>) -> Reader<'a> {
            Reader { reader : reader.get_list(Pointer, ::std::ptr::null()) }
        }
    }

    impl <'a> Reader<'a> {
        pub fn get(self, index : u32) -> data::Reader<'a> {
            assert!(index <  self.len());
            self.reader.get_pointer_element(index).get_data(::std::ptr::null(), 0)
        }
    }

    pub struct Builder<'a> {
        builder : ListBuilder<'a>
    }

    impl <'a> Builder<'a> {
        pub fn new(builder : ListBuilder<'a>) -> Builder<'a> {
            Builder { builder : builder }
        }

        pub fn len(&self) -> u32 { self.builder.len() }

        pub fn set(&mut self, index : u32, value : data::Reader) {
            assert!(index < self.len());
            self.builder.get_pointer_element(index).set_data(value);
        }

        pub fn borrow<'b>(&'b mut self) -> Builder<'b> {
            Builder {builder : self.builder}
        }
    }


    impl <'a> FromPointerBuilder<'a> for Builder<'a> {
        fn init_pointer(builder : PointerBuilder<'a>, size : u32) -> Builder<'a> {
            Builder {
                builder : builder.init_list(Pointer, size)
            }
        }
        fn get_from_pointer(builder : PointerBuilder<'a>) -> Builder<'a> {
            Builder {
                builder : builder.get_list(Pointer, ::std::ptr::null())
            }
        }
    }

    impl <'a> Builder<'a> {
        pub fn get(self, index : u32) -> data::Builder<'a> {
            assert!(index < self.len());
            self.builder.get_pointer_element(index).get_data(::std::ptr::null(), 0)
        }
    }


    impl <'a> ::traits::SetPointerBuilder<Builder<'a>> for Reader<'a> {
        fn set_pointer_builder<'b>(pointer : ::layout::PointerBuilder<'b>, value : Reader<'a>) {
            pointer.set_list(&value.reader);
        }
    }
}
