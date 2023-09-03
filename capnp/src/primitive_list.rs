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

use core::marker;

use crate::introspect;
use crate::private::layout::{
    data_bits_per_element, ListBuilder, ListReader, PointerBuilder, PointerReader, PrimitiveElement,
};
use crate::traits::{FromPointerBuilder, FromPointerReader, IndexMove, ListIter};
use crate::Result;

#[derive(Clone, Copy)]
pub struct Owned<T> {
    marker: marker::PhantomData<T>,
}

impl<T> introspect::Introspect for Owned<T>
where
    T: introspect::Introspect,
{
    fn introspect() -> introspect::Type {
        introspect::Type::list_of(T::introspect())
    }
}

impl<T> crate::traits::Owned for Owned<T>
where
    T: PrimitiveElement + introspect::Introspect,
{
    type Reader<'a> = Reader<'a, T>;
    type Builder<'a> = Builder<'a, T>;
}

#[derive(Clone, Copy)]
pub struct Reader<'a, T>
where
    T: PrimitiveElement,
{
    marker: marker::PhantomData<T>,
    reader: ListReader<'a>,
}

impl<'a, T: PrimitiveElement> Reader<'a, T> {
    pub fn len(&self) -> u32 {
        self.reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(self) -> ListIter<Reader<'a, T>, T> {
        let l = self.len();
        ListIter::new(self, l)
    }
}

impl<'a, T: PrimitiveElement> FromPointerReader<'a> for Reader<'a, T> {
    fn get_from_pointer(
        reader: &PointerReader<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Reader<'a, T>> {
        Ok(Reader {
            reader: reader.get_list(T::element_size(), default)?,
            marker: marker::PhantomData,
        })
    }
}

impl<'a, T: PrimitiveElement> IndexMove<u32, T> for Reader<'a, T> {
    fn index_move(&self, index: u32) -> T {
        self.get(index)
    }
}

impl<'a, T: PrimitiveElement> Reader<'a, T> {
    /// Gets the `T` at position `index`. Panics if `index` is greater than or
    /// equal to `len()`.
    pub fn get(&self, index: u32) -> T {
        assert!(index < self.len());
        PrimitiveElement::get(&self.reader, index)
    }

    /// Gets the `T` at position `index`. Returns `None` if `index`
    /// is greater than or equal to `len()`.
    pub fn try_get(&self, index: u32) -> Option<T> {
        if index < self.len() {
            Some(PrimitiveElement::get(&self.reader, index))
        } else {
            None
        }
    }

    #[cfg(target_endian = "little")]
    /// Returns something if the slice is as expected in memory.
    pub fn as_slice(&self) -> Option<&[T]> {
        if self.reader.get_element_size() == T::element_size() {
            let bytes = self.reader.into_raw_bytes();
            Some(unsafe {
                use core::slice;
                slice::from_raw_parts(
                    bytes.as_ptr() as *mut T,
                    8 * bytes.len() / (data_bits_per_element(T::element_size())) as usize,
                )
            })
        } else {
            None
        }
    }
}

impl<'a, T> crate::traits::IntoInternalListReader<'a> for Reader<'a, T>
where
    T: PrimitiveElement,
{
    fn into_internal_list_reader(self) -> ListReader<'a> {
        self.reader
    }
}

pub struct Builder<'a, T>
where
    T: PrimitiveElement,
{
    marker: marker::PhantomData<T>,
    builder: ListBuilder<'a>,
}

impl<'a, T> Builder<'a, T>
where
    T: PrimitiveElement,
{
    pub fn len(&self) -> u32 {
        self.builder.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn into_reader(self) -> Reader<'a, T> {
        Reader {
            marker: marker::PhantomData,
            reader: self.builder.into_reader(),
        }
    }

    pub fn set(&mut self, index: u32, value: T) {
        assert!(index < self.len());
        PrimitiveElement::set(&self.builder, index, value);
    }

    #[cfg(target_endian = "little")]
    pub fn as_slice(&mut self) -> Option<&mut [T]> {
        if self.builder.get_element_size() == T::element_size() {
            let bytes = self.builder.into_raw_bytes();
            Some(unsafe {
                core::slice::from_raw_parts_mut(
                    bytes.as_mut_ptr() as *mut T,
                    8 * bytes.len() / (data_bits_per_element(T::element_size())) as usize,
                )
            })
        } else {
            None
        }
    }
}

impl<'a, T: PrimitiveElement> FromPointerBuilder<'a> for Builder<'a, T> {
    fn init_pointer(builder: PointerBuilder<'a>, size: u32) -> Builder<'a, T> {
        Builder {
            builder: builder.init_list(T::element_size(), size),
            marker: marker::PhantomData,
        }
    }
    fn get_from_pointer(
        builder: PointerBuilder<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Builder<'a, T>> {
        Ok(Builder {
            builder: builder.get_list(T::element_size(), default)?,
            marker: marker::PhantomData,
        })
    }
}

impl<'a, T: PrimitiveElement> Builder<'a, T> {
    /// Gets the `T` at position `index`. Panics if `index` is greater than or
    /// equal to `len()`.
    pub fn get(&self, index: u32) -> T {
        assert!(index < self.len());
        PrimitiveElement::get_from_builder(&self.builder, index)
    }

    /// Gets the `T` at position `index`. Returns `None` if `index`
    /// is greater than or equal to `len()`.
    pub fn try_get(&self, index: u32) -> Option<T> {
        if index < self.len() {
            Some(PrimitiveElement::get_from_builder(&self.builder, index))
        } else {
            None
        }
    }

    pub fn reborrow(&mut self) -> Builder<'_, T> {
        Builder {
            marker: marker::PhantomData,
            builder: self.builder.reborrow(),
        }
    }
}

impl<'a, T> crate::traits::SetPointerBuilder for Reader<'a, T>
where
    T: PrimitiveElement,
{
    fn set_pointer_builder<'b>(
        mut pointer: PointerBuilder<'b>,
        value: Reader<'a, T>,
        canonicalize: bool,
    ) -> Result<()> {
        pointer.set_list(&value.reader, canonicalize)
    }
}

impl<'a, T> ::core::iter::IntoIterator for Reader<'a, T>
where
    T: PrimitiveElement,
{
    type Item = T;
    type IntoIter = ListIter<Reader<'a, T>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: PrimitiveElement + crate::introspect::Introspect> From<Reader<'a, T>>
    for crate::dynamic_value::Reader<'a>
{
    fn from(t: Reader<'a, T>) -> crate::dynamic_value::Reader<'a> {
        crate::dynamic_value::Reader::List(crate::dynamic_list::Reader::new(
            t.reader,
            T::introspect(),
        ))
    }
}

impl<'a, T: PrimitiveElement + crate::introspect::Introspect> From<Builder<'a, T>>
    for crate::dynamic_value::Builder<'a>
{
    fn from(t: Builder<'a, T>) -> crate::dynamic_value::Builder<'a> {
        crate::dynamic_value::Builder::List(crate::dynamic_list::Builder::new(
            t.builder,
            T::introspect(),
        ))
    }
}
