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

use crate::traits::{FromPointerReader, FromPointerBuilder, ListIter, IndexMove};
use crate::private::arena::{ReaderArena, BuilderArena};
use crate::private::layout::{ListReader, ListBuilder, PointerReader, PointerBuilder, Pointer};
use crate::Result;

#[derive(Clone, Copy)]
pub struct Owned;

impl crate::traits::Owned for Owned {
    type Reader<'a, A: ReaderArena + 'a> = Reader<'a, A>;
    type Builder<'a, A: BuilderArena + 'a> = Builder<'a, A>;
}

#[derive(Clone, Copy)]
pub struct Reader<'a, A> {
    pub reader: ListReader<&'a A>
}

impl <'a, A> Reader<'a, A> where A: ReaderArena {
    pub(crate) fn new<'b>(reader: ListReader<&'b A>) -> Reader<'b, A> {
        Reader { reader: reader }
    }

    pub fn len(&self) -> u32 { self.reader.len() }

    pub fn iter(self) -> ListIter<Reader<'a, A>, Result<crate::any_pointer::Reader<'a, A>>>{
        let l = self.len();
        ListIter::new(self, l)
    }

    pub fn get(self, index : u32) -> crate::any_pointer::Reader<'a, A> {
        assert!(index <  self.len());
        crate::any_pointer::Reader::new(self.reader.get_pointer_element(index))
    }
}

impl <'a, A> IndexMove<u32, Result<crate::any_pointer::Reader<'a, A>>> for Reader<'a, A>
where A: ReaderArena,
      Self: Copy // TODO why is this Copy needed?
{
    fn index_move(&self, index: u32) -> Result<crate::any_pointer::Reader<'a, A>> {
        Ok(self.get(index))
    }
}

impl <'a, A> FromPointerReader<'a, A> for Reader<'a, A> where A: ReaderArena {
    fn get_from_pointer(reader: PointerReader<&'a A>, default: Option<&'a [crate::Word]>) -> Result<Reader<'a, A>> {
        Ok(Reader { reader: reader.get_list(Pointer, default)? })
    }
}

impl <'a, A> crate::traits::IntoInternalListReader<'a, A> for Reader<'a, A> where A: ReaderArena {
    fn into_internal_list_reader(self) -> ListReader<&'a A> {
        self.reader
    }
}

pub struct Builder<'a, A> {
    builder: ListBuilder<&'a mut A>
}

impl <'a, A> Builder<'a, A> where A: BuilderArena {
    pub fn len(&self) -> u32 { self.builder.len() }

    pub fn into_reader(self) -> Reader<'a, A> {
        Reader { reader: self.builder.into_reader() }
    }

    pub fn get(self, index: u32) -> crate::any_pointer::Builder<'a, A> {
        assert!(index <  self.len());
        crate::any_pointer::Builder::new(self.builder.get_pointer_element(index))
    }

    pub fn reborrow<'b>(&'b mut self) -> Builder<'b, A> {
        Builder {builder: self.builder.reborrow()}
    }
}

impl <'a, A> FromPointerBuilder<'a, A> for Builder<'a, A> where A: BuilderArena {
    fn init_pointer(builder: PointerBuilder<&'a mut A>, size : u32) -> Builder<'a, A> {
        Builder {
            builder: builder.init_list(Pointer, size)
        }
    }

    fn get_from_pointer(builder: PointerBuilder<&'a mut A>, default: Option<&'a [crate::Word]>) -> Result<Builder<'a, A>> {
        Ok(Builder {
            builder: builder.get_list(Pointer, default)?
        })
    }
}

impl <'a, A> crate::traits::SetPointerBuilder for Reader<'a, A> where A: ReaderArena {
    fn set_pointer_builder<'b, B>(pointer: PointerBuilder<&'b mut B>,
                                  value: Reader<'a, A>,
                                  canonicalize: bool) -> Result<()>
        where B: BuilderArena
    {
        pointer.set_list(&value.reader, canonicalize)?;
        Ok(())
    }
}

impl <'a, A> core::iter::IntoIterator for Reader<'a, A>
where A: ReaderArena + Copy //TODO why is this Copy needed?
{
    type Item = Result<crate::any_pointer::Reader<'a, A>>;
    type IntoIter = ListIter<Reader<'a, A>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
