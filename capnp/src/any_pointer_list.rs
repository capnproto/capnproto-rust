// Copyright (c) 2018 the capnproto-rust contributors
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

//! List of AnyPointers.
//!
//! Note: this cannot be used for a list of structs, since such lists are not encoded
//! as pointer lists.

use core;
use crate::traits::{FromPointerReader, FromPointerBuilder, ListIter, IndexMove};
use crate::private::layout::{ListReader, ListBuilder, PointerReader, PointerBuilder, Pointer};
use crate::Result;

#[derive(Clone, Copy)]
pub struct Owned;

impl <'a> crate::traits::Owned<'a> for Owned {
    type Reader = Reader<'a>;
    type Builder = Builder<'a>;
}

#[derive(Clone, Copy)]
pub struct Reader<'a> {
    pub reader: ListReader<'a>
}

impl <'a> Reader<'a> {
    pub fn new<'b>(reader: ListReader<'b>) -> Reader<'b> {
        Reader { reader: reader }
    }

    pub fn len(&self) -> u32 { self.reader.len() }

    pub fn iter(self) -> ListIter<Reader<'a>, Result<crate::any_pointer::Reader<'a>>>{
        let l = self.len();
        ListIter::new(self, l)
    }

    pub fn get(self, index : u32) -> crate::any_pointer::Reader<'a> {
        assert!(index <  self.len());
        crate::any_pointer::Reader::new(self.reader.get_pointer_element(index))
    }
}

impl <'a> IndexMove<u32, Result<crate::any_pointer::Reader<'a>>> for Reader<'a>{
    fn index_move(&self, index: u32) -> Result<crate::any_pointer::Reader<'a>> {
        Ok(self.get(index))
    }
}

impl <'a> FromPointerReader<'a> for Reader<'a> {
    fn get_from_pointer(reader: &PointerReader<'a>, default: Option<&'a [crate::Word]>) -> Result<Reader<'a>> {
        Ok(Reader { reader: reader.get_list(Pointer, default)? })
    }
}

impl <'a> crate::traits::IntoInternalListReader<'a> for Reader<'a> {
    fn into_internal_list_reader(self) -> ListReader<'a> {
        self.reader
    }
}

pub struct Builder<'a> {
    builder: ListBuilder<'a>
}

impl <'a> Builder<'a> {
    pub fn new(builder: ListBuilder<'a>) -> Builder<'a> {
        Builder { builder: builder }
    }

    pub fn len(&self) -> u32 { self.builder.len() }

    pub fn into_reader(self) -> Reader<'a> {
        Reader { reader: self.builder.into_reader() }
    }

    pub fn get(self, index : u32) -> crate::any_pointer::Builder<'a> {
        assert!(index <  self.len());
        crate::any_pointer::Builder::new(self.builder.get_pointer_element(index))
    }

    pub fn reborrow<'b>(&'b mut self) -> Builder<'b> {
        Builder {builder: self.builder.borrow()}
    }
}

impl <'a> FromPointerBuilder<'a> for Builder<'a> {
    fn init_pointer(builder: PointerBuilder<'a>, size : u32) -> Builder<'a> {
        Builder {
            builder: builder.init_list(Pointer, size)
        }
    }

    fn get_from_pointer(builder: PointerBuilder<'a>, default: Option<&'a [crate::Word]>) -> Result<Builder<'a>> {
        Ok(Builder {
            builder: builder.get_list(Pointer, default)?
        })
    }
}

impl <'a> crate::traits::SetPointerBuilder<Builder<'a>> for Reader<'a> {
    fn set_pointer_builder<'b>(pointer: PointerBuilder<'b>,
                               value: Reader<'a>,
                               canonicalize: bool) -> Result<()> {
        pointer.set_list(&value.reader, canonicalize)?;
        Ok(())
    }
}

impl <'a> core::iter::IntoIterator for Reader<'a> {
    type Item = Result<crate::any_pointer::Reader<'a>>;
    type IntoIter = ListIter<Reader<'a>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
