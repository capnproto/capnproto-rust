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
use crate::traits::{FromPointerBuilder, FromPointerReader, IndexMove, ListIter, SetterInput};
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

    pub fn reborrow(&self) -> Reader<'_> {
        Reader {
            reader: self.reader,
        }
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

    #[inline]
    pub fn set(&mut self, index: u32, value: impl SetterInput<crate::text::Owned>) {
        assert!(index < self.len());
        SetterInput::set_pointer_builder(
            self.builder.reborrow().get_pointer_element(index),
            value,
            false,
        )
        .unwrap()
        // The text impls of SetterInput never return an error, so
        // the above unwrap() won't panic.
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

impl<'a> crate::traits::SetterInput<Owned> for Reader<'a> {
    #[inline]
    fn set_pointer_builder<'b>(
        mut pointer: crate::private::layout::PointerBuilder<'b>,
        value: Reader<'a>,
        canonicalize: bool,
    ) -> Result<()> {
        pointer.set_list(&value.reader, canonicalize)
    }
}

impl<'a, T: AsRef<str>> crate::traits::SetterInput<Owned> for &'a [T] {
    #[inline]
    fn set_pointer_builder<'b>(
        pointer: PointerBuilder<'b>,
        value: &'a [T],
        _canonicalize: bool,
    ) -> Result<()> {
        let mut builder = pointer.init_list(
            crate::private::layout::ElementSize::Pointer,
            value.len() as u32,
        );
        for (idx, v) in value.iter().enumerate() {
            builder
                .reborrow()
                .get_pointer_element(idx as u32)
                .set_text(v.as_ref().into());
        }
        Ok(())
    }
}

impl<'a, T: AsRef<str>, const N: usize> crate::traits::SetterInput<Owned> for &'a [T; N] {
    #[inline]
    fn set_pointer_builder<'b>(
        pointer: PointerBuilder<'b>,
        value: &'a [T; N],
        canonicalize: bool,
    ) -> Result<()> {
        crate::traits::SetterInput::set_pointer_builder(pointer, &value[..], canonicalize)
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

impl<'a> crate::dynamic_value::DowncastReader<'a> for Reader<'a> {
    fn downcast_reader(v: crate::dynamic_value::Reader<'a>) -> Self {
        let dl: crate::dynamic_list::Reader = v.downcast();
        assert!(dl
            .element_type()
            .loose_equals(crate::introspect::TypeVariant::Text.into()));
        Reader { reader: dl.reader }
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

impl<'a> crate::dynamic_value::DowncastBuilder<'a> for Builder<'a> {
    fn downcast_builder(v: crate::dynamic_value::Builder<'a>) -> Self {
        let dl: crate::dynamic_list::Builder = v.downcast();
        assert!(dl
            .element_type()
            .loose_equals(crate::introspect::TypeVariant::Text.into()));
        Builder {
            builder: dl.builder,
        }
    }
}

impl core::fmt::Debug for Reader<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(
            &::core::convert::Into::<crate::dynamic_value::Reader<'_>>::into(*self),
            f,
        )
    }
}
