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

//! List of primitives.

use core::{marker};

use crate::traits::{FromPointerReader, FromPointerBuilder, IndexMove, ListIter};
use crate::private::arena::{BuilderArena, ReaderArena};
use crate::private::layout::{ListReader, ListBuilder, PointerReader, PointerBuilder,
                             PrimitiveElement};
use crate::Result;

#[derive(Clone, Copy)]
pub struct Owned<T> {
    marker: marker::PhantomData<T>,
}

impl <T> crate::traits::Owned for Owned<T> where T: PrimitiveElement {
    type Reader<'a, A: ReaderArena + 'a> = Reader<'a, A, T>;
    type Builder<'a, A: BuilderArena + 'a> = Builder<'a, A, T>;
}

#[derive(Clone, Copy)]
pub struct Reader<'a, A, T> where T: PrimitiveElement {
    marker: marker::PhantomData<T>,
    reader: ListReader<&'a A>
}

impl <'a, A, T: PrimitiveElement> Reader<'a, A, T> where A: ReaderArena {
    pub fn len(&self) -> u32 { self.reader.len() }

    pub fn iter(self) -> ListIter<Self, T>{
        let l = self.len();
        ListIter::new(self, l)
    }
}

impl <'a, A, T: PrimitiveElement> FromPointerReader<'a, A> for Reader<'a, A, T> where A: ReaderArena {
    fn get_from_pointer(reader: PointerReader<&'a A>, default: Option<&'a [crate::Word]>) -> Result<Reader<'a, A, T>> {
        Ok(Reader { reader: reader.get_list(T::element_size(), default)?,
                    marker: marker::PhantomData })
    }
}

impl <'a, A, T: PrimitiveElement>  IndexMove<u32, T> for Reader<'a, A, T> where A: ReaderArena {
    fn index_move(&self, index: u32) -> T {
        self.get(index)
    }
}

impl <'a, A, T: PrimitiveElement> Reader<'a, A, T> where A: ReaderArena {
    pub fn get(&self, index: u32) -> T {
        assert!(index < self.len());
        PrimitiveElement::get(&self.reader, index)
    }
}

impl <'a, A, T> crate::traits::IntoInternalListReader<'a, A> for Reader<'a, A, T> where T: PrimitiveElement {
    fn into_internal_list_reader(self) -> ListReader<&'a A> {
        self.reader
    }
}

pub struct Builder<'a, A, T> where T: PrimitiveElement {
    marker: marker::PhantomData<T>,
    builder: ListBuilder<&'a mut A>
}

impl <'a, A, T> Builder<'a, A, T> where T: PrimitiveElement, A: BuilderArena {
    pub fn len(&self) -> u32 { self.builder.len() }

    pub fn into_reader(self) -> Reader<'a, A, T> {
        Reader {
            marker: marker::PhantomData,
            reader: self.builder.into_reader(),
        }
    }

    pub fn set(&mut self, index: u32, value: T) {
        PrimitiveElement::set(&self.builder, index, value);
    }
}

impl <'a, A, T: PrimitiveElement> FromPointerBuilder<'a, A> for Builder<'a, A, T> where A: BuilderArena {
    fn init_pointer(builder: PointerBuilder<&'a mut A>, size: u32) -> Builder<'a, A, T> {
        Builder { builder: builder.init_list(T::element_size(), size),
                  marker: marker::PhantomData }
    }
    fn get_from_pointer(builder: PointerBuilder<&'a mut A>, default: Option<&'a [crate::Word]>) -> Result<Builder<'a, A, T>> {
        Ok(Builder { builder: builder.get_list(T::element_size(), default)?,
                     marker: marker::PhantomData })
    }
}

impl <'a, A, T: PrimitiveElement> Builder<'a, A, T> where A: BuilderArena {
    pub fn get(&self, index: u32) -> T {
        assert!(index < self.len());
        PrimitiveElement::get_from_builder(&self.builder, index)
    }

    pub fn reborrow<'b>(&'b mut self) -> Builder<'b, A, T> {
        Builder { builder: self.builder.reborrow(),
                  .. *self }
    }
}

impl <'a, A, T> crate::traits::SetPointerBuilder for Reader<'a, A, T>
where T: PrimitiveElement,
      A: ReaderArena
{
    fn set_pointer_builder<'b, B>(pointer: PointerBuilder<&'b mut B>,
                                  value: Reader<'a, A, T>,
                                  canonicalize: bool) -> Result<()>
        where B: BuilderArena
    {
        pointer.set_list(&value.reader, canonicalize)
    }
}

impl <'a, A, T> ::core::iter::IntoIterator for Reader<'a, A, T>
where T: PrimitiveElement,
      A: ReaderArena
{
    type Item = T;
    type IntoIter = ListIter<Reader<'a, A, T>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
