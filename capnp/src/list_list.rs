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

//! List of lists.

use crate::traits::{FromPointerReader, FromPointerBuilder, ListIter, IndexMove};

use crate::private::arena::{ReaderArena, BuilderArena};
use crate::private::layout::{ListReader, ListBuilder, PointerReader, PointerBuilder, Pointer};
use crate::Result;

#[derive(Clone, Copy)]
pub struct Owned<T> where T: crate::traits::Owned {
    marker: ::core::marker::PhantomData<T>,
}

impl<T> crate::traits::Owned for Owned<T> where T: crate::traits::Owned {
    type Reader<'a, A: ReaderArena + 'a>  = Reader<'a, A, T>;
    type Builder<'a, A: BuilderArena + 'a> = Builder<'a, A, T>;
}

pub struct Reader<'a, A, T> where T: crate::traits::Owned, A: ReaderArena {
    marker: ::core::marker::PhantomData<<T as crate::traits::Owned>::Reader<'a, A>>,
    reader: ListReader<&'a A>,
}

impl <'a, A, T> Reader<'a, A, T> where T: crate::traits::Owned, A: ReaderArena {
    pub fn len(&self) -> u32 { self.reader.len() }
    pub fn iter(self) -> ListIter<Reader<'a, A, T>, Result<<T as crate::traits::Owned>::Reader<'a, A>>> {
        ListIter::new(self, self.len())
    }
}

impl <'a, A, T> Clone for Reader<'a, A, T> where T: crate::traits::Owned, A: ReaderArena {
    fn clone(&self) -> Reader<'a, A, T> {
        Reader { marker : self.marker, reader : self.reader }
    }
}

impl <'a, A, T> Copy for Reader<'a, A, T> where T: crate::traits::Owned, A: ReaderArena {}

impl <'a, A, T>  IndexMove<u32, Result<<T as crate::traits::Owned>::Reader<'a, A>>> for Reader<'a, A, T>
where T: crate::traits::Owned,
      A: ReaderArena
{
    fn index_move(&self, index: u32) -> Result<<T as crate::traits::Owned>::Reader<'a, A>> {
        self.get(index)
    }
}

impl <'a, A, T> FromPointerReader<'a, A> for Reader<'a, A, T> where T: crate::traits::Owned, A: ReaderArena {
    fn get_from_pointer(reader: PointerReader<&'a A>, default: Option<&'a [crate::Word]>) -> Result<Reader<'a, A, T>> {
        Ok(Reader { reader: reader.get_list(Pointer, default)?,
                    marker: ::core::marker::PhantomData })
    }
}

impl <'a, A, T> Reader<'a, A, T> where T: crate::traits::Owned, A: ReaderArena {
    pub fn get(self, index: u32) -> Result<<T as crate::traits::Owned>::Reader<'a, A>> {
        assert!(index < self.len());
        FromPointerReader::get_from_pointer(self.reader.get_pointer_element(index), None)
    }
}

pub struct Builder<'a, A, T> where T: crate::traits::Owned, A: BuilderArena {
    marker: ::core::marker::PhantomData<T>,
    builder: ListBuilder<&'a mut A>,
}

impl <'a, A, T> Builder<'a, A, T> where T: crate::traits::Owned, A: BuilderArena {
    pub fn len(&self) -> u32 { self.builder.len() }

    pub fn into_reader(self) -> Reader<'a, A, T> {
        Reader { reader: self.builder.into_reader(), marker: ::core::marker::PhantomData }
    }
}

impl <'a, A, T> Builder<'a, A, T> where T: crate::traits::Owned, A: BuilderArena {
    pub fn init(self, index: u32, size: u32) -> <T as crate::traits::Owned>::Builder<'a, A> {
        FromPointerBuilder::init_pointer(self.builder.get_pointer_element(index), size)
    }
}

impl <'a, A, T> Builder<'a, A, T> where T: crate::traits::Owned, A: BuilderArena {
    pub fn reborrow<'b>(&'b mut self) -> Builder<'b, A, T> {
        Builder {builder: self.builder.reborrow(), marker: ::core::marker::PhantomData}
    }
}

impl <'a, A, T> FromPointerBuilder<'a, A> for Builder<'a, A, T> where T: crate::traits::Owned, A: BuilderArena {
    fn init_pointer(builder: PointerBuilder<&'a mut A>, size: u32) -> Builder<'a, A, T> {
        Builder {
            marker: ::core::marker::PhantomData,
            builder: builder.init_list(Pointer, size)
        }
    }
    fn get_from_pointer(builder: PointerBuilder<&'a mut A>, default: Option<&'a [crate::Word]>) -> Result<Builder<'a, A, T>> {
        Ok(Builder {
            marker: ::core::marker::PhantomData,
            builder: builder.get_list(Pointer, default)?
        })
    }
}

impl <'a, A, T> Builder<'a, A, T> where T: crate::traits::Owned, A: BuilderArena {
    pub fn get(self, index: u32) -> Result<<T as crate::traits::Owned>::Builder<'a, A>> {
        assert!(index < self.len());
        FromPointerBuilder::get_from_pointer(self.builder.get_pointer_element(index), None)
    }
}

impl <'a, A, T> crate::traits::SetPointerBuilder for Reader<'a, A, T>
where T: crate::traits::Owned,
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
where T: crate::traits::Owned,
      A: ReaderArena
{
    type Item = Result<<T as crate::traits::Owned>::Reader<'a, A>>;
    type IntoIter = ListIter<Reader<'a, A, T>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
