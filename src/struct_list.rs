// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

//! List of structs.

use private::layout::{ListReader, ListBuilder, PointerReader, PointerBuilder, InlineComposite};
use traits::{FromPointerReader, FromPointerBuilder,
             FromStructBuilder, FromStructReader, HasStructSize,
             IndexMove, ListIter};
use Result;

#[derive(Copy, Clone)]
pub struct Owned<T> where T: for<'a> ::traits::OwnedStruct<'a> {
    marker: ::std::marker::PhantomData<T>,
}

impl<'a, T> ::traits::Owned<'a> for Owned<T> where T: for<'b> ::traits::OwnedStruct<'b> {
    type Reader = Reader<'a, T>;
    type Builder = Builder<'a, T>;
}

pub struct Reader<'a, T> where T: for<'b> ::traits::OwnedStruct<'b> {
    marker: ::std::marker::PhantomData<T>,
    reader: ListReader<'a>
}

impl <'a, T> Clone for Reader<'a, T> where T: for<'b> ::traits::OwnedStruct<'b> {
    fn clone(&self) -> Reader<'a, T> {
        Reader { marker : self.marker, reader : self.reader }
    }
}
impl <'a, T> Copy for Reader<'a, T> where T: for<'b> ::traits::OwnedStruct<'b> {}

impl <'a, T> Reader<'a, T> where T: for<'b> ::traits::OwnedStruct<'b> {
    pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
        Reader::<'b, T> { reader : reader, marker : ::std::marker::PhantomData }
    }

    pub fn len(&self) -> u32 { self.reader.len() }

    pub fn iter(self) -> ListIter<Reader<'a, T>, <T as ::traits::OwnedStruct<'a>>::Reader> {
        ListIter::new(self, self.len())
    }
}

impl <'a, T> Reader<'a, T> where T: for<'b> ::traits::OwnedStruct<'b> {
    pub fn borrow<'b>(&'b self) -> Reader<'b, T>  {
        Reader {reader : self.reader, marker : ::std::marker::PhantomData}
    }
}

impl <'a, T> FromPointerReader<'a> for Reader<'a, T> where T: for<'b> ::traits::OwnedStruct<'b> {
    fn get_from_pointer(reader : &PointerReader<'a>) -> Result<Reader<'a, T>> {
        Ok(Reader { reader : try!(reader.get_list(InlineComposite, ::std::ptr::null())),
                    marker : ::std::marker::PhantomData })
    }
}

impl <'a, T>  IndexMove<u32, <T as ::traits::OwnedStruct<'a>>::Reader> for Reader<'a, T>
where T: for<'b> ::traits::OwnedStruct<'b> {
    fn index_move(&self, index : u32) -> <T as ::traits::OwnedStruct<'a>>::Reader {
        self.get(index)
    }
}

impl <'a, T> Reader<'a, T> where T: for<'b> ::traits::OwnedStruct<'b> {
    pub fn get(self, index: u32) -> <T as ::traits::OwnedStruct<'a>>::Reader {
        assert!(index < self.len());
        FromStructReader::new(self.reader.get_struct_element(index))
    }
}

pub struct Builder<'a, T> where T: for<'b> ::traits::OwnedStruct<'b> {
    marker : ::std::marker::PhantomData<T>,
    builder : ListBuilder<'a>
}

impl <'a, T> Builder<'a, T> where T: for<'b> ::traits::OwnedStruct<'b> {
    pub fn new(builder : ListBuilder<'a>) -> Builder<'a, T> {
        Builder { builder : builder, marker : ::std::marker::PhantomData }
    }

    pub fn len(&self) -> u32 { self.builder.len() }

    //        pub fn set(&self, index : uint, value : T) {
    //        }

}

impl <'a, T> Builder<'a, T> where T: for<'b> ::traits::OwnedStruct<'b> {
    pub fn borrow<'b>(&'b mut self) -> Builder<'b, T> {
        Builder {builder : self.builder, marker : ::std::marker::PhantomData}
    }
}

impl <'a, T> FromPointerBuilder<'a> for Builder<'a, T> where T: for<'b> ::traits::OwnedStruct<'b> {
    fn init_pointer(builder : PointerBuilder<'a>, size : u32) -> Builder<'a, T> {
        Builder {
            marker : ::std::marker::PhantomData,
            builder : builder.init_struct_list(
                size,
                <<T as ::traits::OwnedStruct>::Builder as HasStructSize>::struct_size())
        }
    }
    fn get_from_pointer(builder : PointerBuilder<'a>) -> Result<Builder<'a, T>> {
        Ok(Builder {
            marker : ::std::marker::PhantomData,
            builder :
            try!(builder.get_struct_list(<<T as ::traits::OwnedStruct>::Builder as HasStructSize>::struct_size(),
                                         ::std::ptr::null()))
        })
    }
}

impl <'a, T> Builder<'a, T> where T: for<'b> ::traits::OwnedStruct<'b> {
    pub fn get(self, index: u32) -> <T as ::traits::OwnedStruct<'a>>::Builder {
        assert!(index < self.len());
        FromStructBuilder::new(self.builder.get_struct_element(index))
    }
}

impl <'a, T> ::traits::SetPointerBuilder<Builder<'a, T>> for Reader<'a, T>
    where T: for<'b> ::traits::OwnedStruct<'b>
{
    fn set_pointer_builder<'b>(pointer : ::private::layout::PointerBuilder<'b>,
                               value : Reader<'a, T>) -> Result<()> {
        pointer.set_list(&value.reader)
    }
}
