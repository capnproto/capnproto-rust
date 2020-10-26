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
use crate::private::arena::BuilderArena;
use crate::private::layout::{ListReader, ListBuilder, PointerReader, PointerBuilder,
                             PrimitiveElement};
use crate::Result;

#[derive(Clone, Copy)]
pub struct Owned<T> {
    marker: marker::PhantomData<T>,
}

impl <'a, T, A: 'a> crate::traits::Owned<'a, A> for Owned<T> where T: PrimitiveElement {
    type Reader = Reader<T, &'a A>;
    type Builder = Builder<T, &'a mut A>;
}

#[derive(Clone, Copy)]
pub struct Reader<T, A> where T: PrimitiveElement {
    marker: marker::PhantomData<T>,
    reader: ListReader<A>
}

impl <T: PrimitiveElement, A> Reader<T, A> {
    pub fn new(reader: ListReader<A>) -> Reader<T, A> {
        Reader { reader: reader, marker: marker::PhantomData }
    }

    pub fn len(&self) -> u32 { self.reader.len() }

    pub fn iter(self) -> ListIter<Reader<T, A>, T>{
        let l = self.len();
        ListIter::new(self, l)
    }
}

impl <'a, T: PrimitiveElement, A> FromPointerReader<'a, A> for Reader<T, &'a A> {
    fn get_from_pointer(reader: &PointerReader<&'a A>, default: Option<&'a [crate::Word]>) -> Result<Reader<T, &'a A>> {
        Ok(Reader { reader: reader.get_list(T::element_size(), default)?,
                    marker: marker::PhantomData })
    }
}

impl <'a, T: PrimitiveElement, A>  IndexMove<u32, T> for Reader<T, &'a A> {
    fn index_move(&self, index: u32) -> T {
        self.get(index)
    }
}

impl <T: PrimitiveElement, A> Reader<T, A> {
    pub fn get(&self, index: u32) -> T {
        assert!(index < self.len());
        PrimitiveElement::get(&self.reader, index)
    }
}

impl <'a, T, A> crate::traits::IntoInternalListReader<'a, A> for Reader<T, &'a A> where T: PrimitiveElement {
    fn into_internal_list_reader(self) -> ListReader<&'a A> {
        self.reader
    }
}

pub struct Builder<T, A> where T: PrimitiveElement {
    marker: marker::PhantomData<T>,
    builder: ListBuilder<A>
}

impl <'a, T, A> Builder<T, &'a mut A> where T: PrimitiveElement, A: BuilderArena {
    pub fn new(builder: ListBuilder<&'a mut A>) -> Builder<T, &'a mut A> {
        Builder { builder: builder, marker: marker::PhantomData }
    }

    pub fn len(&self) -> u32 { self.builder.len() }

    pub fn into_reader(self) -> Reader<T, &'a A> {
        Reader {
            marker: marker::PhantomData,
            reader: self.builder.into_reader(),
        }
    }

    pub fn set(&mut self, index: u32, value: T) {
        PrimitiveElement::set(&self.builder, index, value);
    }
}

impl <'a, T: PrimitiveElement, A> FromPointerBuilder<'a, A> for Builder<T, &'a mut A> {
    fn init_pointer(builder: PointerBuilder<&'a mut A>, size: u32) -> Builder<T, &'a mut A> {
        Builder { builder: builder.init_list(T::element_size(), size),
                  marker: marker::PhantomData }
    }
    fn get_from_pointer(builder: PointerBuilder<&'a mut A>, default: Option<&'a [crate::Word]>) -> Result<Builder<T, &'a mut A>> {
        Ok(Builder { builder: builder.get_list(T::element_size(), default)?,
                     marker: marker::PhantomData })
    }
}

impl <'a, T: PrimitiveElement, A> Builder<T, &'a mut A> {
    pub fn get(&self, index: u32) -> T {
        assert!(index < self.len());
        PrimitiveElement::get_from_builder(&self.builder, index)
    }

    pub fn reborrow<'b>(&'b self) -> Builder<T, &'b mut A> {
        Builder { .. *self }
    }
}

impl <T, A> crate::traits::SetPointerBuilder<Builder<T, A>> for Reader<T, A>
    where T: PrimitiveElement
{
    fn set_pointer_builder<'b, B>(pointer: PointerBuilder<&'b mut B>,
                                  value: Reader<T, A>,
                                  canonicalize: bool) -> Result<()> {
        pointer.set_list(&value.reader, canonicalize)
    }
}

impl <T, A> ::core::iter::IntoIterator for Reader<T, A>
    where T: PrimitiveElement
{
    type Item = T;
    type IntoIter = ListIter<Reader<T, A>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
