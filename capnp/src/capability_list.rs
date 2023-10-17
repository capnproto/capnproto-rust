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
#![cfg(feature = "alloc")]

use alloc::boxed::Box;
use core::marker::PhantomData;

use crate::capability::FromClientHook;
use crate::private::capability::ClientHook;
use crate::private::layout::{ListBuilder, ListReader, Pointer, PointerBuilder, PointerReader};
use crate::traits::{FromPointerBuilder, FromPointerReader, IndexMove, ListIter};
use crate::Result;

#[derive(Copy, Clone)]
pub struct Owned<T>
where
    T: FromClientHook,
{
    marker: PhantomData<T>,
}

impl<T> crate::introspect::Introspect for Owned<T>
where
    T: FromClientHook,
{
    fn introspect() -> crate::introspect::Type {
        crate::introspect::Type::list_of(crate::introspect::TypeVariant::Capability.into())
    }
}

impl<T> crate::traits::Owned for Owned<T>
where
    T: FromClientHook,
{
    type Reader<'a> = Reader<'a, T>;
    type Builder<'a> = Builder<'a, T>;
}

pub struct Reader<'a, T>
where
    T: FromClientHook,
{
    marker: PhantomData<T>,
    reader: ListReader<'a>,
}

impl<'a, T> Clone for Reader<'a, T>
where
    T: FromClientHook,
{
    fn clone(&self) -> Reader<'a, T> {
        *self
    }
}
impl<'a, T> Copy for Reader<'a, T> where T: FromClientHook {}

impl<'a, T> Reader<'a, T>
where
    T: FromClientHook,
{
    pub fn len(&self) -> u32 {
        self.reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(self) -> ListIter<Reader<'a, T>, Result<T>> {
        ListIter::new(self, self.len())
    }
}

impl<'a, T> Reader<'a, T>
where
    T: FromClientHook,
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
    T: FromClientHook,
{
    fn get_from_pointer(
        reader: &PointerReader<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Reader<'a, T>> {
        Ok(Reader {
            reader: reader.get_list(Pointer, default)?,
            marker: PhantomData,
        })
    }
}

impl<'a, T> Reader<'a, T>
where
    T: FromClientHook,
{
    /// Gets the element at position `index`. Panics if `index` is greater than or
    /// equal to `len()`.
    pub fn get(self, index: u32) -> Result<T> {
        assert!(index < self.len());
        Ok(FromClientHook::new(
            self.reader.get_pointer_element(index).get_capability()?,
        ))
    }

    /// Gets the element at position `index`. Returns `None` if `index`
    /// is greater than or equal to `len()`.
    pub fn try_get(self, index: u32) -> Option<Result<T>> {
        if index < self.len() {
            Some(
                self.reader
                    .get_pointer_element(index)
                    .get_capability()
                    .map(FromClientHook::new),
            )
        } else {
            None
        }
    }
}

impl<'a, T> IndexMove<u32, Result<T>> for Reader<'a, T>
where
    T: FromClientHook,
{
    fn index_move(&self, index: u32) -> Result<T> {
        self.get(index)
    }
}

pub struct Builder<'a, T>
where
    T: FromClientHook,
{
    marker: PhantomData<T>,
    builder: ListBuilder<'a>,
}

impl<'a, T> Builder<'a, T>
where
    T: FromClientHook,
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

    pub fn set(&mut self, index: u32, value: Box<dyn ClientHook>) {
        assert!(index < self.len());
        self.builder
            .reborrow()
            .get_pointer_element(index)
            .set_capability(value);
    }
}

impl<'a, T> Builder<'a, T>
where
    T: FromClientHook,
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
    T: FromClientHook,
{
    fn init_pointer(builder: PointerBuilder<'a>, size: u32) -> Builder<'a, T> {
        Builder {
            marker: PhantomData,
            builder: builder.init_list(Pointer, size),
        }
    }
    fn get_from_pointer(
        builder: PointerBuilder<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Builder<'a, T>> {
        Ok(Builder {
            marker: PhantomData,
            builder: builder.get_list(Pointer, default)?,
        })
    }
}

impl<'a, T> Builder<'a, T>
where
    T: FromClientHook,
{
    /// Gets the element at position `index`. Panics if `index` is greater than or
    /// equal to `len()`.
    pub fn get(self, index: u32) -> Result<T> {
        assert!(index < self.len());
        Ok(FromClientHook::new(
            self.builder.get_pointer_element(index).get_capability()?,
        ))
    }

    /// Gets the element at position `index`. Returns `None` if `index`
    /// is greater than or equal to `len()`.
    pub fn try_get(self, index: u32) -> Option<Result<T>> {
        if index < self.len() {
            Some(
                self.builder
                    .get_pointer_element(index)
                    .get_capability()
                    .map(FromClientHook::new),
            )
        } else {
            None
        }
    }
}

impl<'a, T> crate::traits::SetPointerBuilder for Reader<'a, T>
where
    T: FromClientHook,
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
    T: FromClientHook,
{
    type Item = Result<T>;
    type IntoIter = ListIter<Reader<'a, T>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: FromClientHook> From<Reader<'a, T>> for crate::dynamic_value::Reader<'a> {
    fn from(t: Reader<'a, T>) -> crate::dynamic_value::Reader<'a> {
        crate::dynamic_value::Reader::List(crate::dynamic_list::Reader::new(
            t.reader,
            crate::introspect::TypeVariant::Capability.into(),
        ))
    }
}

impl<'a, T: FromClientHook> From<Builder<'a, T>> for crate::dynamic_value::Builder<'a> {
    fn from(t: Builder<'a, T>) -> crate::dynamic_value::Builder<'a> {
        crate::dynamic_value::Builder::List(crate::dynamic_list::Builder::new(
            t.builder,
            crate::introspect::TypeVariant::Capability.into(),
        ))
    }
}
