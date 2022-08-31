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

use core::{convert, str, ops};

use crate::{Error, Result};

#[derive(Copy, Clone)]
pub struct Owned(());

impl<'a> crate::traits::Owned<'a> for Owned {
    type Reader = Reader<'a>;
    type Builder = Builder<'a>;
}

pub type Reader<'a> = &'a str;

pub fn new_reader(v : &[u8]) -> Result<Reader<'_>> {
    match str::from_utf8(v) {
        Ok(v) => Ok(v),
        Err(e) => Err(Error::failed(
            format!("Text contains non-utf8 data: {:?}", e))),
    }
}

impl <'a> crate::traits::FromPointerReader<'a> for Reader<'a> {
    fn get_from_pointer(reader: &crate::private::layout::PointerReader<'a>,
                        default: Option<&'a [crate::Word]>) -> Result<Reader<'a>> {
        reader.get_text(default)
    }
}

pub struct Builder<'a> {
    bytes: &'a mut [u8],
    pos: usize,
}

impl <'a> Builder <'a> {
    pub fn new(bytes: &mut [u8], pos: u32) -> Result<Builder<'_>> {
        if pos != 0 {
            if let Err(e) = str::from_utf8(bytes) {
                return Err(Error::failed(
                    format!("Text contains non-utf8 data: {:?}", e)))
            }
        }
        Ok(Builder { bytes, pos: pos as usize })
    }

    pub fn push_ascii(&mut self, ascii: u8) {
        assert!(ascii < 128);
        self.bytes[self.pos] = ascii;
        self.pos += 1;
    }

    pub fn push_str(&mut self, string: &str) {
        let bytes = string.as_bytes();
        self.bytes[self.pos..(self.pos+bytes.len())].copy_from_slice(bytes);
        self.pos += bytes.len();
    }

    pub fn clear(&mut self) {
        for b in &mut self.bytes[..self.pos] {
            *b = 0;
        }
        self.pos = 0;
    }
}

impl <'a> ops::Deref for Builder <'a> {
    type Target = str;
    fn deref(&self) -> &str {
        str::from_utf8(self.bytes)
            .expect("text::Builder contents are checked for utf8-validity upon construction")
    }
}

impl <'a> ops::DerefMut for Builder <'a> {
    fn deref_mut(&mut self) -> &mut str {
        str::from_utf8_mut(self.bytes)
            .expect("text::Builder contents are checked for utf8-validity upon construction")
    }
}

impl <'a> convert::AsRef<str> for Builder<'a> {
    fn as_ref(&self) -> &str {
        str::from_utf8(self.bytes)
            .expect("text::Builder contents are checked for utf8-validity upon construction")
    }
}

impl <'a> crate::traits::FromPointerBuilder<'a> for Builder<'a> {
    fn init_pointer(builder: crate::private::layout::PointerBuilder<'a>, size: u32) -> Builder<'a> {
        builder.init_text(size)
    }
    fn get_from_pointer(builder: crate::private::layout::PointerBuilder<'a>, default: Option<&'a [crate::Word]>) -> Result<Builder<'a>> {
        builder.get_text(default)
    }
}

impl <'a> crate::traits::SetPointerBuilder for Reader<'a> {
    fn set_pointer_builder<'b>(pointer: crate::private::layout::PointerBuilder<'b>,
                               value: Reader<'a>,
                               _canonicalize: bool)
                               -> Result<()>
    {
        pointer.set_text(value);
        Ok(())
    }
}
