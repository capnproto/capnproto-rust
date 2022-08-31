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

//! List of enums.

use crate::traits::{FromPointerReader, FromPointerBuilder,
                    ToU16, FromU16, ListIter, IndexMove};
use crate::private::layout::{ListReader, ListBuilder, PointerReader, PointerBuilder,
                             TwoBytes, PrimitiveElement};
use crate::{NotInSchema, Result};

use core::marker::PhantomData;

#[derive(Clone, Copy)]
pub struct Owned<T> {
    marker: PhantomData<T>,
}

impl <'a, T> crate::traits::Owned<'a> for Owned<T> where T: FromU16 {
    type Reader = Reader<'a, T>;
    type Builder = Builder<'a, T>;
}

#[derive(Clone, Copy)]
pub struct Reader<'a, T> {
    marker: PhantomData<T>,
    reader: ListReader<'a>
}

impl <'a, T: FromU16> Reader<'a, T> {
    pub fn len(&self) -> u32 { self.reader.len() }

    pub fn iter(self) -> ListIter<Reader<'a, T>, ::core::result::Result<T, NotInSchema>>{
        let l = self.len();
        ListIter::new(self, l)
    }
}

impl <'a, T : FromU16> FromPointerReader<'a> for Reader<'a, T> {
    fn get_from_pointer(reader: &PointerReader<'a>, default: Option<&'a [crate::Word]>) -> Result<Reader<'a, T>> {
        Ok(Reader { reader: reader.get_list(TwoBytes, default)?,
                    marker: PhantomData })
    }
}

impl <'a, T: FromU16>  IndexMove<u32, ::core::result::Result<T, NotInSchema>> for Reader<'a, T>{
    fn index_move(&self, index: u32) -> ::core::result::Result<T, NotInSchema> {
        self.get(index)
    }
}

impl <'a, T : FromU16> Reader<'a, T> {
    /// Gets the `T` at position `index`. Panics if `index` is greater than or
    /// equal to `len()`.
    pub fn get(&self, index: u32) -> ::core::result::Result<T, NotInSchema> {
        assert!(index < self.len());
        let result: u16 = PrimitiveElement::get(&self.reader, index);
        FromU16::from_u16(result)
    }

    /// Gets the `T` at position `index`. Returns `None` if `index`
    /// is greater than or equal to `len()`.
    pub fn try_get(&self, index: u32) -> Option<::core::result::Result<T, NotInSchema>> {
        if index < self.len() {
            let result: u16 = PrimitiveElement::get(&self.reader, index);
            Some(FromU16::from_u16(result))
        } else {
            None
        }
    }
}

impl <'a, T> crate::traits::IntoInternalListReader<'a> for Reader<'a, T> where T: PrimitiveElement {
    fn into_internal_list_reader(self) -> ListReader<'a> {
        self.reader
    }
}

pub struct Builder<'a, T> {
    marker: PhantomData<T>,
    builder: ListBuilder<'a>
}

impl <'a, T : ToU16 + FromU16> Builder<'a, T> {
    pub fn len(&self) -> u32 { self.builder.len() }

    pub fn into_reader(self) -> Reader<'a, T> {
        Reader { reader: self.builder.into_reader(), marker: PhantomData, }
    }

    pub fn set(&mut self, index: u32, value: T) {
        assert!(index < self.len());
        PrimitiveElement::set(&self.builder, index, value.to_u16());
    }
}

impl <'a, T : FromU16> FromPointerBuilder<'a> for Builder<'a, T> {
    fn init_pointer(builder: PointerBuilder<'a>, size: u32) -> Builder<'a, T> {
        Builder { builder: builder.init_list(TwoBytes, size),
                  marker: PhantomData }
    }
    fn get_from_pointer(builder: PointerBuilder<'a>, default: Option<&'a [crate::Word]>) -> Result<Builder<'a, T>> {
        Ok(Builder { builder: builder.get_list(TwoBytes, default)?,
                     marker: PhantomData })
    }
}

impl <'a, T : ToU16 + FromU16>  Builder<'a, T> {
    /// Gets the `T` at position `index`. Panics if `index` is greater than or
    /// equal to `len()`.
    pub fn get(&self, index: u32) -> ::core::result::Result<T, NotInSchema> {
        assert!(index < self.len());
        let result: u16 = PrimitiveElement::get_from_builder(&self.builder, index);
        FromU16::from_u16(result)
    }

    /// Gets the `T` at position `index`. Returns `None` if `index`
    /// is greater than or equal to `len()`.
    pub fn try_get(&self, index: u32) -> Option<::core::result::Result<T, NotInSchema>> {
        if index < self.len() {
            let result: u16 = PrimitiveElement::get_from_builder(&self.builder, index);
            Some(FromU16::from_u16(result))
        } else {
            None
        }
    }

    pub fn reborrow(&self) -> Builder<'_, T> {
        Builder { .. *self }
    }
}

impl <'a, T> crate::traits::SetPointerBuilder for Reader<'a, T> {
    fn set_pointer_builder<'b>(pointer: crate::private::layout::PointerBuilder<'b>,
                               value: Reader<'a, T>,
                               canonicalize: bool) -> Result<()> {
        pointer.set_list(&value.reader, canonicalize)
    }
}

impl <'a, T: FromU16> ::core::iter::IntoIterator for Reader<'a, T> {
    type Item = ::core::result::Result<T, NotInSchema>;
    type IntoIter = ListIter<Reader<'a, T>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
