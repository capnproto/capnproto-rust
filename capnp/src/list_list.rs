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

use crate::introspect;
use crate::private::layout::{ListBuilder, ListReader, Pointer, PointerBuilder, PointerReader};
use crate::traits::{FromPointerBuilder, FromPointerReader, IndexMove, ListIter};
use crate::Result;

#[derive(Clone, Copy)]
pub struct Owned<T>
where
    T: crate::traits::Owned,
{
    marker: ::core::marker::PhantomData<T>,
}

impl<T> introspect::Introspect for Owned<T>
where
    T: introspect::Introspect + crate::traits::Owned,
{
    fn introspect() -> introspect::Type {
        introspect::Type::list_of(T::introspect())
    }
}

impl<T> crate::traits::Owned for Owned<T>
where
    T: crate::traits::Owned,
{
    type Reader<'a> = Reader<'a, T>;
    type Builder<'a> = Builder<'a, T>;
}

pub struct Reader<'a, T>
where
    T: crate::traits::Owned,
{
    marker: ::core::marker::PhantomData<T::Reader<'a>>,
    reader: ListReader<'a>,
}

impl<'a, T> Reader<'a, T>
where
    T: crate::traits::Owned,
{
    pub fn len(&self) -> u32 {
        self.reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(self) -> ListIter<Reader<'a, T>, Result<T::Reader<'a>>> {
        ListIter::new(self, self.len())
    }
}

impl<'a, T> Clone for Reader<'a, T>
where
    T: crate::traits::Owned,
{
    fn clone(&self) -> Reader<'a, T> {
        *self
    }
}

impl<'a, T> Copy for Reader<'a, T> where T: crate::traits::Owned {}

impl<'a, T> IndexMove<u32, Result<T::Reader<'a>>> for Reader<'a, T>
where
    T: crate::traits::Owned,
{
    fn index_move(&self, index: u32) -> Result<T::Reader<'a>> {
        self.get(index)
    }
}

impl<'a, T> FromPointerReader<'a> for Reader<'a, T>
where
    T: crate::traits::Owned,
{
    fn get_from_pointer(
        reader: &PointerReader<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Reader<'a, T>> {
        Ok(Reader {
            reader: reader.get_list(Pointer, default)?,
            marker: ::core::marker::PhantomData,
        })
    }
}

impl<'a, T> Reader<'a, T>
where
    T: crate::traits::Owned,
{
    /// Gets the element at position `index`. Panics if `index` is greater than or
    /// equal to `len()`.
    pub fn get(self, index: u32) -> Result<T::Reader<'a>> {
        assert!(index < self.len());
        FromPointerReader::get_from_pointer(&self.reader.get_pointer_element(index), None)
    }

    /// Gets the element at position `index`. Returns `None` if `index`
    /// is greater than or equal to `len()`.
    pub fn try_get(self, index: u32) -> Option<Result<T::Reader<'a>>> {
        if index < self.len() {
            Some(FromPointerReader::get_from_pointer(
                &self.reader.get_pointer_element(index),
                None,
            ))
        } else {
            None
        }
    }
}

impl<'a, T> crate::traits::IntoInternalListReader<'a> for Reader<'a, T>
where
    T: crate::traits::Owned,
{
    fn into_internal_list_reader(self) -> ListReader<'a> {
        self.reader
    }
}

pub struct Builder<'a, T>
where
    T: crate::traits::Owned,
{
    marker: ::core::marker::PhantomData<T>,
    builder: ListBuilder<'a>,
}

impl<'a, T> Builder<'a, T>
where
    T: crate::traits::Owned,
{
    pub fn len(&self) -> u32 {
        self.builder.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn into_reader(self) -> Reader<'a, T> {
        Reader {
            reader: self.builder.into_reader(),
            marker: ::core::marker::PhantomData,
        }
    }
}

impl<'a, T> Builder<'a, T>
where
    T: crate::traits::Owned,
{
    pub fn init(self, index: u32, size: u32) -> T::Builder<'a> {
        FromPointerBuilder::init_pointer(self.builder.get_pointer_element(index), size)
    }
}

impl<'a, T> Builder<'a, T>
where
    T: crate::traits::Owned,
{
    pub fn reborrow(&mut self) -> Builder<'_, T> {
        Builder {
            builder: self.builder.reborrow(),
            marker: ::core::marker::PhantomData,
        }
    }
}

impl<'a, T> FromPointerBuilder<'a> for Builder<'a, T>
where
    T: crate::traits::Owned,
{
    fn init_pointer(builder: PointerBuilder<'a>, size: u32) -> Builder<'a, T> {
        Builder {
            marker: ::core::marker::PhantomData,
            builder: builder.init_list(Pointer, size),
        }
    }
    fn get_from_pointer(
        builder: PointerBuilder<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Builder<'a, T>> {
        Ok(Builder {
            marker: ::core::marker::PhantomData,
            builder: builder.get_list(Pointer, default)?,
        })
    }
}

impl<'a, T> Builder<'a, T>
where
    T: crate::traits::Owned,
{
    /// Gets the element at position `index`. Panics if `index` is greater than or
    /// equal to `len()`.
    pub fn get(self, index: u32) -> Result<T::Builder<'a>> {
        assert!(index < self.len());
        FromPointerBuilder::get_from_pointer(self.builder.get_pointer_element(index), None)
    }

    /// Gets the element at position `index`. Returns `None` if `index`
    /// is greater than or equal to `len()`.
    pub fn try_get(self, index: u32) -> Option<Result<T::Builder<'a>>> {
        if index < self.len() {
            Some(FromPointerBuilder::get_from_pointer(
                self.builder.get_pointer_element(index),
                None,
            ))
        } else {
            None
        }
    }

    pub fn set<'b>(&mut self, index: u32, value: T::Reader<'a>) -> Result<()>
    where
        T::Reader<'a>: crate::traits::IntoInternalListReader<'b>,
    {
        use crate::traits::IntoInternalListReader;
        assert!(index < self.len());
        self.builder
            .reborrow()
            .get_pointer_element(index)
            .set_list(&value.into_internal_list_reader(), false)
    }
}

impl<'a, T> crate::traits::SetPointerBuilder for Reader<'a, T>
where
    T: crate::traits::Owned,
{
    fn set_pointer_builder<'b>(
        mut pointer: crate::private::layout::PointerBuilder<'b>,
        value: Reader<'a, T>,
        canonicalize: bool,
    ) -> Result<()> {
        pointer.set_list(&value.reader, canonicalize)
    }
}

impl<'a, T> ::core::iter::IntoIterator for Reader<'a, T>
where
    T: crate::traits::Owned,
{
    type Item = Result<T::Reader<'a>>;
    type IntoIter = ListIter<Reader<'a, T>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: crate::traits::Owned> From<Reader<'a, T>> for crate::dynamic_value::Reader<'a> {
    fn from(t: Reader<'a, T>) -> crate::dynamic_value::Reader<'a> {
        crate::dynamic_value::Reader::List(crate::dynamic_list::Reader::new(
            t.reader,
            T::introspect(),
        ))
    }
}

impl<'a, T: crate::traits::Owned> From<Builder<'a, T>> for crate::dynamic_value::Builder<'a> {
    fn from(t: Builder<'a, T>) -> crate::dynamic_value::Builder<'a> {
        crate::dynamic_value::Builder::List(crate::dynamic_list::Builder::new(
            t.builder,
            T::introspect(),
        ))
    }
}
