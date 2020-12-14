// Copyright (c) 2017 David Renshaw and contributors
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

//! List of capabilities.

use alloc::boxed::Box;
use core::marker::PhantomData;

use crate::capability::{FromClientHook};
use crate::private::arena::{ReaderArena, BuilderArena};
use crate::private::capability::ClientHook;
use crate::private::layout::{ListReader, ListBuilder, PointerReader, PointerBuilder, Pointer};
use crate::traits::{FromPointerReader, FromPointerBuilder, IndexMove, ListIter};
use crate::Result;

#[derive(Copy, Clone)]
pub struct Owned<T> where T: FromClientHook {
    marker: PhantomData<T>,
}

impl<T> crate::traits::Owned for Owned<T> where T: FromClientHook {
    type Reader<'a, A: ReaderArena + 'a> = Reader<'a, A, T>;
    type Builder<'a, A: BuilderArena + 'a> = Builder<'a, A, T>;
}

pub struct Reader<'a, A, T> where T: FromClientHook, A: ReaderArena {
    marker: PhantomData<T>,
    reader: ListReader<&'a A>,
}

impl <'a, A, T> Clone for Reader<'a, A, T> where T: FromClientHook, A: ReaderArena {
    fn clone(&self) -> Reader<'a, A, T> {
        Reader { marker : self.marker, reader : self.reader }
    }
}
impl <'a, A, T> Copy for Reader<'a, A, T> where T: FromClientHook, A: ReaderArena {}

impl <'a, A, T> Reader<'a, A, T> where T: FromClientHook, A: ReaderArena {
    pub fn len(&self) -> u32 { self.reader.len() }

    pub fn iter(self) -> ListIter<Reader<'a, A, T>, Result<T>> {
        ListIter::new(self, self.len())
    }
}

impl <'a, A, T> Reader<'a, A, T> where T: FromClientHook, A: ReaderArena {
    pub fn reborrow<'b>(&'b self) -> Reader<'b, A, T>  {
        Reader { reader: self.reader, marker: PhantomData }
    }
}

impl <'a, A, T> FromPointerReader<'a, A> for Reader<'a, A, T> where T: FromClientHook, A: ReaderArena {
    fn get_from_pointer(reader: PointerReader<&'a A>, default: Option<&'a [crate::Word]>) -> Result<Reader<'a, A, T>> {
        Ok(Reader { reader: reader.get_list(Pointer, default)?,
                    marker: PhantomData })
    }
}

impl <'a, A, T> Reader<'a, A, T> where T: FromClientHook, A: ReaderArena {
    pub fn get(self, index: u32) -> Result<T> {
        assert!(index < self.len());
        Ok(FromClientHook::new(self.reader.get_pointer_element(index).get_capability()?))
    }
}

impl <'a, A, T> IndexMove<u32, Result<T>> for Reader<'a, A, T> where T: FromClientHook, A: ReaderArena {
    fn index_move(&self, index: u32) -> Result<T> {
        self.get(index)
    }
}

pub struct Builder<'a, A, T> where T: FromClientHook, A: BuilderArena {
    marker: PhantomData<T>,
    builder: ListBuilder<&'a mut A>,
}

impl <'a, A, T> Builder<'a, A, T> where T: FromClientHook, A: BuilderArena {
    pub fn len(&self) -> u32 { self.builder.len() }

    pub fn into_reader(self) -> Reader<'a, A, T> {
        Reader {
            marker: PhantomData,
            reader: self.builder.into_reader(),
        }
    }

    pub fn set(&mut self, index: u32, value: Box<dyn ClientHook>) {
        assert!(index < self.len());
        self.builder.reborrow().get_pointer_element(index).set_capability(value);
    }
}

impl <'a, A, T> Builder<'a, A, T> where T: FromClientHook, A: BuilderArena {
    pub fn reborrow<'b>(&'b mut self) -> Builder<'b, A, T> {
        Builder { builder: self.builder.reborrow(), marker: PhantomData }
    }
}

impl <'a, A, T> FromPointerBuilder<'a, A> for Builder<'a, A, T> where T: FromClientHook, A: BuilderArena {
    fn init_pointer(builder: PointerBuilder<&'a mut A>, size: u32) -> Builder<'a, A, T> {
        Builder {
            marker: PhantomData,
            builder: builder.init_list(Pointer, size),
        }
    }
    fn get_from_pointer(builder: PointerBuilder<&'a mut A>, default: Option<&'a [crate::Word]>) -> Result<Builder<'a, A, T>> {
        Ok(Builder {
            marker: PhantomData,
            builder: builder.get_list(Pointer, default)?
        })
    }
}

impl <'a, A, T> Builder<'a, A, T> where T: FromClientHook, A: BuilderArena {
    pub fn get(self, index: u32) -> Result<T> {
        assert!(index < self.len());
        Ok(FromClientHook::new(self.builder.get_pointer_element(index).get_capability()?))
    }
}

impl <'a, A, T> crate::traits::SetPointerBuilder for Reader<'a, A, T>
where T: FromClientHook,
      A: ReaderArena
{
    fn set_pointer_builder<'b, B>(pointer: crate::private::layout::PointerBuilder<&'b mut B>,
                                  value: Reader<'a, A, T>,
                                  canonicalize: bool) -> Result<()>
        where B: BuilderArena
    {
        pointer.set_list(&value.reader, canonicalize)
    }
}

impl <'a, A, T> ::core::iter::IntoIterator for Reader<'a, A, T>
where T: FromClientHook,
      A: ReaderArena
{
    type Item = Result<T>;
    type IntoIter = ListIter<Reader<'a, A, T>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

