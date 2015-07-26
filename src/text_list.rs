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

//! List of strings containing UTF-8 encoded text.

use traits::{FromPointerReader, FromPointerBuilder};
use private::layout::{ListBuilder, ListReader, Pointer, PointerBuilder, PointerReader};
use Result;

#[derive(Copy, Clone)]
pub struct Owned;

impl <'a> ::traits::Owned<'a> for Owned {
    type Reader = Reader<'a>;
    type Builder = Builder<'a>;
}

#[derive(Clone, Copy)]
pub struct Reader<'a> {
    reader: ListReader<'a>
}

impl <'a> Reader<'a> {
    pub fn new<'b>(reader : ListReader<'b>) -> Reader<'b> {
        Reader::<'b> { reader : reader }
    }

    pub fn len(&self) -> u32 { self.reader.len() }
}

impl <'a> FromPointerReader<'a> for Reader<'a> {
    fn get_from_pointer(reader: &PointerReader<'a>) -> Result<Reader<'a>> {
        Ok(Reader { reader : try!(reader.get_list(Pointer, ::std::ptr::null())) })
    }
}

impl <'a> Reader<'a> {
    pub fn get(self, index : u32) -> Result<::text::Reader<'a>> {
        assert!(index <  self.len());
        self.reader.get_pointer_element(index).get_text(::std::ptr::null(), 0)
    }
}

pub struct Builder<'a> {
    builder: ListBuilder<'a>
}

impl <'a> Builder<'a> {
    pub fn new(builder : ListBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
    }

    pub fn len(&self) -> u32 { self.builder.len() }

    pub fn set(&mut self, index : u32, value : ::text::Reader) {
        assert!(index < self.len());
        self.builder.get_pointer_element(index).set_text(value);
    }

    pub fn borrow<'b>(&'b mut self) -> Builder<'b> {
        Builder::<'b> {builder : self.builder}
    }
}


impl <'a> FromPointerBuilder<'a> for Builder<'a> {
    fn init_pointer(builder : PointerBuilder<'a>, size : u32) -> Builder<'a> {
        Builder {
            builder : builder.init_list(Pointer, size)
        }
    }
    fn get_from_pointer(builder : PointerBuilder<'a>) -> Result<Builder<'a>> {
        Ok(Builder {
            builder : try!(builder.get_list(Pointer, ::std::ptr::null()))
        })
    }
}

impl <'a> Builder<'a> {
    pub fn get(self, index : u32) -> Result<::text::Builder<'a>> {
        self.builder.get_pointer_element(index).get_text(::std::ptr::null(), 0)
    }
}

impl <'a> ::traits::SetPointerBuilder<Builder<'a>> for Reader<'a> {
    fn set_pointer_builder<'b>(pointer : ::private::layout::PointerBuilder<'b>,
                               value : Reader<'a>) -> Result<()> {
        pointer.set_list(&value.reader)
    }
}

