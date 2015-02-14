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

pub struct Reader<'a, T> {
    reader : ListReader<'a>
}

impl <'a, T> Copy for Reader<'a, T> {}

impl <'a, T : FromStructReader<'a>> Reader<'a, T> {
    pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b, T> {
        Reader::<'b, T> { reader : reader }
    }

    pub fn len(&self) -> u32 { self.reader.len() }

    pub fn iter(self) -> ListIter<Reader<'a, T>, T> {
        return ListIter::new(self, self.len());
    }
}

impl <'a, T> Reader<'a, T>  {
    pub fn borrow<'b, U>(&'b self) -> Reader<'b, U> where T : ::traits::CastableTo<U> {
        Reader {reader : self.reader}
    }
}


impl <'a, T : FromStructReader<'a>> FromPointerReader<'a> for Reader<'a, T> {
    fn get_from_pointer(reader : &PointerReader<'a>) -> Reader<'a, T> {
        Reader { reader : reader.get_list(InlineComposite, ::std::ptr::null()) }
    }
}

impl <'a, T : FromStructReader<'a>>  IndexMove<u32, T> for Reader<'a, T> {
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

impl <'a, T> Builder<'a, T> {
    pub fn borrow<'b, U>(&'b mut self) -> Builder<'b, U> where T : ::traits::CastableTo<U> {
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
    fn set_pointer_builder<'b>(pointer : ::private::layout::PointerBuilder<'b>, value : Reader<'a, T>) {
        pointer.set_list(&value.reader);
    }
}

impl <'a, 'b : 'a, T, U : ::traits::CastableTo<T>> ::traits::CastableTo<Builder<'a, T> > for Builder<'b, U> {
    fn cast(self) -> Builder<'a, T> {
        Builder { builder : self.builder }
    }
}
