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

use core::marker::PhantomData;

use crate::introspect;
use crate::private::layout::{
    InlineComposite, ListBuilder, ListReader, PointerBuilder, PointerReader,
};
use crate::traits::{FromPointerBuilder, FromPointerReader, HasStructSize, IndexMove, ListIter};
use crate::Result;

#[derive(Copy, Clone)]
pub struct Owned<T>
where
    T: crate::traits::OwnedStruct,
{
    marker: PhantomData<T>,
}

impl<T> introspect::Introspect for Owned<T>
where
    T: introspect::Introspect + crate::traits::OwnedStruct,
{
    fn introspect() -> introspect::Type {
        introspect::Type::list_of(T::introspect())
    }
}

impl<T> crate::traits::Owned for Owned<T>
where
    T: crate::traits::OwnedStruct,
{
    type Reader<'a> = Reader<'a, T>;
    type Builder<'a> = Builder<'a, T>;
}

pub struct Reader<'a, T>
where
    T: crate::traits::OwnedStruct,
{
    marker: PhantomData<T>,
    reader: ListReader<'a>,
}

impl<'a, T> Clone for Reader<'a, T>
where
    T: crate::traits::OwnedStruct,
{
    fn clone(&self) -> Reader<'a, T> {
        *self
    }
}
impl<'a, T> Copy for Reader<'a, T> where T: crate::traits::OwnedStruct {}

impl<'a, T> Reader<'a, T>
where
    T: crate::traits::OwnedStruct,
{
    pub fn len(&self) -> u32 {
        self.reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(self) -> ListIter<Reader<'a, T>, T::Reader<'a>> {
        ListIter::new(self, self.len())
    }
}

impl<'a, T> Reader<'a, T>
where
    T: crate::traits::OwnedStruct,
{
    pub fn reborrow(&self) -> Reader<'_, T> {
        Reader {
            reader: self.reader,
            marker: PhantomData,
        }
    }
}

impl<'a, T> FromPointerReader<'a> for Reader<'a, T>
where
    T: crate::traits::OwnedStruct,
{
    fn get_from_pointer(
        reader: &PointerReader<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Reader<'a, T>> {
        Ok(Reader {
            reader: reader.get_list(InlineComposite, default)?,
            marker: PhantomData,
        })
    }
}

impl<'a, T> IndexMove<u32, T::Reader<'a>> for Reader<'a, T>
where
    T: crate::traits::OwnedStruct,
{
    fn index_move(&self, index: u32) -> T::Reader<'a> {
        self.get(index)
    }
}

impl<'a, T> Reader<'a, T>
where
    T: crate::traits::OwnedStruct,
{
    /// Gets the element at position `index`. Panics if `index` is greater than or
    /// equal to `len()`.
    pub fn get(self, index: u32) -> T::Reader<'a> {
        assert!(index < self.len());
        self.reader.get_struct_element(index).into()
    }

    /// Gets the element at position `index`. Returns `None` if `index`
    /// is greater than or equal to `len()`.
    pub fn try_get(self, index: u32) -> Option<T::Reader<'a>> {
        if index < self.len() {
            Some(self.reader.get_struct_element(index).into())
        } else {
            None
        }
    }
}

impl<'a, T> crate::traits::IntoInternalListReader<'a> for Reader<'a, T>
where
    T: crate::traits::OwnedStruct,
{
    fn into_internal_list_reader(self) -> ListReader<'a> {
        self.reader
    }
}

pub struct Builder<'a, T>
where
    T: crate::traits::OwnedStruct,
{
    marker: PhantomData<T>,
    builder: ListBuilder<'a>,
}

impl<'a, T> Builder<'a, T>
where
    T: crate::traits::OwnedStruct,
{
    pub fn len(&self) -> u32 {
        self.builder.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn into_reader(self) -> Reader<'a, T> {
        Reader {
            marker: PhantomData,
            reader: self.builder.into_reader(),
        }
    }

    /// Sets the list element, with the following limitation based on the fact that structs in a
    /// struct list are allocated inline: if the source struct is larger than the target struct
    /// (as can happen if it was created with a newer version of the schema), then it will be
    /// truncated, losing fields.
    pub fn set_with_caveats<'b>(&mut self, index: u32, value: T::Reader<'b>) -> Result<()>
    where
        T::Reader<'b>: crate::traits::IntoInternalStructReader<'b>,
    {
        assert!(index < self.len());
        use crate::traits::IntoInternalStructReader;
        self.builder
            .reborrow()
            .get_struct_element(index)
            .copy_content_from(&value.into_internal_struct_reader())
    }
}

impl<'a, T> Builder<'a, T>
where
    T: crate::traits::OwnedStruct,
{
    pub fn reborrow(&mut self) -> Builder<'_, T> {
        Builder {
            builder: self.builder.reborrow(),
            marker: PhantomData,
        }
    }
}

impl<'a, T> FromPointerBuilder<'a> for Builder<'a, T>
where
    T: crate::traits::OwnedStruct,
{
    fn init_pointer(builder: PointerBuilder<'a>, size: u32) -> Builder<'a, T> {
        Builder {
            marker: PhantomData,
            builder: builder.init_struct_list(size, T::Builder::STRUCT_SIZE),
        }
    }
    fn get_from_pointer(
        builder: PointerBuilder<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Builder<'a, T>> {
        Ok(Builder {
            marker: PhantomData,
            builder: builder.get_struct_list(T::Builder::STRUCT_SIZE, default)?,
        })
    }
}

impl<'a, T> Builder<'a, T>
where
    T: crate::traits::OwnedStruct,
{
    /// Gets the element at position `index`. Panics if `index` is greater than or
    /// equal to `len()`.
    pub fn get(self, index: u32) -> T::Builder<'a> {
        assert!(index < self.len());
        self.builder.get_struct_element(index).into()
    }

    /// Gets the element at position `index`. Returns `None` if `index`
    /// is greater than or equal to `len()`.
    pub fn try_get(self, index: u32) -> Option<T::Builder<'a>> {
        if index < self.len() {
            Some(self.builder.get_struct_element(index).into())
        } else {
            None
        }
    }
}

impl<'a, T> crate::traits::SetPointerBuilder for Reader<'a, T>
where
    T: crate::traits::OwnedStruct,
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
    T: crate::traits::OwnedStruct,
{
    type Item = T::Reader<'a>;
    type IntoIter = ListIter<Reader<'a, T>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: crate::traits::OwnedStruct> From<Reader<'a, T>> for crate::dynamic_value::Reader<'a> {
    fn from(t: Reader<'a, T>) -> crate::dynamic_value::Reader<'a> {
        crate::dynamic_value::Reader::List(crate::dynamic_list::Reader::new(
            t.reader,
            T::introspect(),
        ))
    }
}

impl<'a, T: crate::traits::OwnedStruct> From<Builder<'a, T>> for crate::dynamic_value::Builder<'a> {
    fn from(t: Builder<'a, T>) -> crate::dynamic_value::Builder<'a> {
        crate::dynamic_value::Builder::List(crate::dynamic_list::Builder::new(
            t.builder,
            T::introspect(),
        ))
    }
}
