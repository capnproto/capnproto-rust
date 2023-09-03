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

//! UTF-8 encoded text.

use core::str;

use crate::Result;

#[derive(Copy, Clone)]
pub struct Owned(());

impl crate::traits::Owned for Owned {
    type Reader<'a> = Reader<'a>;
    type Builder<'a> = Builder<'a>;
}

impl crate::introspect::Introspect for Owned {
    fn introspect() -> crate::introspect::Type {
        crate::introspect::TypeVariant::Text.into()
    }
}

/// Wrapper around utf-8 encoded text.
/// This is defined as a tuple struct to allow pattern matching
/// on it via byte literals (for example `text::Reader(b"hello")`).
#[derive(Copy, Clone, PartialEq)]
pub struct Reader<'a>(pub &'a [u8]);

impl<'a> core::cmp::PartialEq<&'a str> for Reader<'a> {
    fn eq(&self, other: &&'a str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<'a> core::cmp::PartialEq<Reader<'a>> for &'a str {
    fn eq(&self, other: &Reader<'a>) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl<'a> core::fmt::Debug for Reader<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.to_str() {
            Ok(s) => write!(f, "{:?}", s),
            Err(_) => write!(f, "<invalid utf-8: {:?}>", self.as_bytes()),
        }
    }
}

impl<'a> From<&'a str> for Reader<'a> {
    fn from(value: &'a str) -> Self {
        Self(value.as_bytes())
    }
}

impl<'a> From<&'a [u8]> for Reader<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self(value)
    }
}

impl<'a, const N: usize> From<&'a [u8; N]> for Reader<'a> {
    fn from(value: &'a [u8; N]) -> Self {
        Self(&value[..])
    }
}

impl<'a> TryFrom<Reader<'a>> for &'a str {
    type Error = core::str::Utf8Error;
    fn try_from(value: Reader<'a>) -> core::result::Result<&'a str, core::str::Utf8Error> {
        let Reader(v) = value;
        str::from_utf8(v)
    }
}

impl<'a> crate::traits::FromPointerReader<'a> for Reader<'a> {
    fn get_from_pointer(
        reader: &crate::private::layout::PointerReader<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Reader<'a>> {
        reader.get_text(default)
    }
}

impl<'a> Reader<'a> {
    /// The string's length, in bytes.
    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_bytes(self) -> &'a [u8] {
        let Self(d) = self;
        d
    }

    /// Converts to a `str`, returning a error if the data contains invalid utf-8.
    pub fn to_str(self) -> core::result::Result<&'a str, core::str::Utf8Error> {
        let Self(s) = self;
        str::from_utf8(s)
    }

    #[cfg(feature = "alloc")]
    /// Converts to a `String`, returning a error if the data contains invalid utf-8.
    pub fn to_string(self) -> core::result::Result<alloc::string::String, core::str::Utf8Error> {
        Ok(self.to_str()?.into())
    }
}

pub struct Builder<'a> {
    /// Does not include the trailing null byte.
    bytes: &'a mut [u8],

    /// Position at which `push_ascii()` and `push_str()` will write to.
    pos: usize,
}

impl<'a> core::cmp::PartialEq for Builder<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl<'a> core::cmp::PartialEq<&'a str> for Builder<'a> {
    fn eq(&self, other: &&'a str) -> bool {
        self.bytes == other.as_bytes()
    }
}

impl<'a> core::cmp::PartialEq<Builder<'a>> for &'a str {
    fn eq(&self, other: &Builder<'a>) -> bool {
        self.as_bytes() == other.bytes
    }
}

impl<'a> Builder<'a> {
    pub fn new(bytes: &mut [u8]) -> Builder<'_> {
        Builder { bytes, pos: 0 }
    }

    pub fn with_pos(bytes: &mut [u8], pos: usize) -> Builder<'_> {
        Builder { bytes, pos }
    }

    /// The string's length, in bytes.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_bytes(self) -> &'a [u8] {
        self.bytes
    }

    /// Converts to a `str`, returning a error if the data contains invalid utf-8.
    pub fn to_str(self) -> core::result::Result<&'a str, core::str::Utf8Error> {
        str::from_utf8(self.bytes)
    }

    #[cfg(feature = "alloc")]
    /// Converts to a `String`, returning a error if the data contains invalid utf-8.
    pub fn to_string(self) -> core::result::Result<alloc::string::String, core::str::Utf8Error> {
        Ok(self.to_str()?.into())
    }

    pub fn as_bytes_mut(self) -> &'a mut [u8] {
        &mut self.bytes[..]
    }

    /// Writes a single ascii character at position `pos` and increments `pos`.
    pub fn push_ascii(&mut self, ascii: u8) {
        assert!(ascii < 128);
        self.bytes[self.pos] = ascii;
        self.pos += 1;
    }

    /// Writes a string at position `pos` and increases `pos` a corresponding amount.
    pub fn push_str(&mut self, string: &str) {
        let bytes = string.as_bytes();
        self.bytes[self.pos..(self.pos + bytes.len())].copy_from_slice(bytes);
        self.pos += bytes.len();
    }

    /// Zeroes all data and resets `pos`.
    pub fn clear(&mut self) {
        for b in &mut self.bytes[..self.pos] {
            *b = 0;
        }
        self.pos = 0;
    }

    pub fn reborrow(&mut self) -> Builder<'_> {
        Builder {
            bytes: self.bytes,
            pos: self.pos,
        }
    }

    pub fn into_reader(self) -> Reader<'a> {
        Reader(self.bytes)
    }

    pub fn reborrow_as_reader(&self) -> Reader<'_> {
        Reader(self.bytes)
    }
}

impl<'a> core::fmt::Debug for Builder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.reborrow_as_reader().to_str() {
            Ok(s) => write!(f, "{:?}", s),
            Err(_) => write!(f, "<invalid utf-8>"),
        }
    }
}

impl<'a> crate::traits::FromPointerBuilder<'a> for Builder<'a> {
    fn init_pointer(builder: crate::private::layout::PointerBuilder<'a>, size: u32) -> Builder<'a> {
        builder.init_text(size)
    }
    fn get_from_pointer(
        builder: crate::private::layout::PointerBuilder<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Builder<'a>> {
        builder.get_text(default)
    }
}

impl<'a> crate::traits::SetPointerBuilder for Reader<'a> {
    fn set_pointer_builder<'b>(
        mut pointer: crate::private::layout::PointerBuilder<'b>,
        value: Reader<'a>,
        _canonicalize: bool,
    ) -> Result<()> {
        pointer.set_text(value);
        Ok(())
    }
}

// Extra impl to make any_pointer::Builder::set_as() and similar methods work
// more smoothly.
impl<'a> crate::traits::SetPointerBuilder for &'a str {
    fn set_pointer_builder<'b>(
        mut pointer: crate::private::layout::PointerBuilder<'b>,
        value: &'a str,
        _canonicalize: bool,
    ) -> Result<()> {
        pointer.set_text(value.into());
        Ok(())
    }
}

impl<'a> From<Reader<'a>> for crate::dynamic_value::Reader<'a> {
    fn from(t: Reader<'a>) -> crate::dynamic_value::Reader<'a> {
        crate::dynamic_value::Reader::Text(t)
    }
}

impl<'a> From<&'a str> for crate::dynamic_value::Reader<'a> {
    fn from(t: &'a str) -> crate::dynamic_value::Reader<'a> {
        crate::dynamic_value::Reader::Text(t.into())
    }
}

impl<'a> From<Builder<'a>> for crate::dynamic_value::Builder<'a> {
    fn from(t: Builder<'a>) -> crate::dynamic_value::Builder<'a> {
        crate::dynamic_value::Builder::Text(t)
    }
}
