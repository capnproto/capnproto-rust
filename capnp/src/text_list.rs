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

//! List of strings containing UTF-8 encoded text.

use crate::private::layout::{ListBuilder, ListReader, Pointer, PointerBuilder, PointerReader};
use crate::traits::{FromPointerBuilder, FromPointerReader, IndexMove, ListIter};
use crate::Result;

#[derive(Copy, Clone)]
pub struct Owned;

impl crate::traits::Owned for Owned {
    type Reader<'a> = Reader<'a>;
    type Builder<'a> = Builder<'a>;
}

impl crate::introspect::Introspect for Owned {
    fn introspect() -> crate::introspect::Type {
        crate::introspect::Type::list_of(crate::introspect::TypeVariant::Text.into())
    }
}

#[derive(Clone, Copy)]
pub struct Reader<'a> {
    reader: ListReader<'a>,
}

impl<'a> Reader<'a> {
    pub fn new<'b>(reader: ListReader<'b>) -> Reader<'b> {
        Reader::<'b> { reader }
    }

    pub fn len(&self) -> u32 {
        self.reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(self) -> ListIter<Reader<'a>, Result<crate::text::Reader<'a>>> {
        let l = self.len();
        ListIter::new(self, l)
    }
}

impl<'a> FromPointerReader<'a> for Reader<'a> {
    fn get_from_pointer(
        reader: &PointerReader<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Reader<'a>> {
        Ok(Reader {
            reader: reader.get_list(Pointer, default)?,
        })
    }
}

impl<'a> IndexMove<u32, Result<crate::text::Reader<'a>>> for Reader<'a> {
    fn index_move(&self, index: u32) -> Result<crate::text::Reader<'a>> {
        self.get(index)
    }
}

impl<'a> Reader<'a> {
    /// Gets the `text::Reader` at position `index`. Panics if `index` is
    /// greater than or equal to `len()`.
    pub fn get(self, index: u32) -> Result<crate::text::Reader<'a>> {
        assert!(index < self.len());
        self.reader.get_pointer_element(index).get_text(None)
    }

    /// Gets the `text::Reader` at position `index`. Returns `None` if `index`
    /// is greater than or equal to `len()`.
    pub fn try_get(self, index: u32) -> Option<Result<crate::text::Reader<'a>>> {
        if index < self.len() {
            Some(self.reader.get_pointer_element(index).get_text(None))
        } else {
            None
        }
    }
}

impl<'a> crate::traits::IntoInternalListReader<'a> for Reader<'a> {
    fn into_internal_list_reader(self) -> ListReader<'a> {
        self.reader
    }
}

pub struct Builder<'a> {
    builder: ListBuilder<'a>,
}

impl<'a> Builder<'a> {
    pub fn new(builder: ListBuilder<'a>) -> Builder<'a> {
        Builder { builder }
    }

    pub fn len(&self) -> u32 {
        self.builder.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set(&mut self, index: u32, value: crate::text::Reader) {
        assert!(index < self.len());
        self.builder
            .reborrow()
            .get_pointer_element(index)
            .set_text(value);
    }

    pub fn into_reader(self) -> Reader<'a> {
        Reader {
            reader: self.builder.into_reader(),
        }
    }

    pub fn reborrow<'b>(&'b mut self) -> Builder<'b> {
        Builder::<'b> {
            builder: self.builder.reborrow(),
        }
    }
}

impl<'a> FromPointerBuilder<'a> for Builder<'a> {
    fn init_pointer(builder: PointerBuilder<'a>, size: u32) -> Builder<'a> {
        Builder {
            builder: builder.init_list(Pointer, size),
        }
    }
    fn get_from_pointer(
        builder: PointerBuilder<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Builder<'a>> {
        Ok(Builder {
            builder: builder.get_list(Pointer, default)?,
        })
    }
}

impl<'a> Builder<'a> {
    /// Gets the `text::Builder` at position `index`. Panics if `index` is
    /// greater than or equal to `len()`.
    pub fn get(self, index: u32) -> Result<crate::text::Builder<'a>> {
        assert!(index < self.len());
        self.builder.get_pointer_element(index).get_text(None)
    }

    /// Gets the `text::Builder` at position `index`. Returns `None` if `index`
    /// is greater than or equal to `len()`.
    pub fn try_get(self, index: u32) -> Option<Result<crate::text::Builder<'a>>> {
        if index < self.len() {
            Some(self.builder.get_pointer_element(index).get_text(None))
        } else {
            None
        }
    }
}

impl<'a> crate::traits::SetPointerBuilder for Reader<'a> {
    fn set_pointer_builder<'b>(
        mut pointer: crate::private::layout::PointerBuilder<'b>,
        value: Reader<'a>,
        canonicalize: bool,
    ) -> Result<()> {
        pointer.set_list(&value.reader, canonicalize)
    }
}

impl<'a> ::core::iter::IntoIterator for Reader<'a> {
    type Item = Result<crate::text::Reader<'a>>;
    type IntoIter = ListIter<Reader<'a>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> From<Reader<'a>> for crate::dynamic_value::Reader<'a> {
    fn from(t: Reader<'a>) -> crate::dynamic_value::Reader<'a> {
        crate::dynamic_value::Reader::List(crate::dynamic_list::Reader {
            reader: t.reader,
            element_type: crate::introspect::TypeVariant::Text.into(),
        })
    }
}

impl<'a> From<Builder<'a>> for crate::dynamic_value::Builder<'a> {
    fn from(t: Builder<'a>) -> crate::dynamic_value::Builder<'a> {
        crate::dynamic_value::Builder::List(crate::dynamic_list::Builder {
            builder: t.builder,
            element_type: crate::introspect::TypeVariant::Text.into(),
        })
    }
}
