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

//! Sequence of bytes.

use crate::private::arena::{BuilderArena, ReaderArena};
use crate::private::layout::{PointerBuilder, PointerReader};
use crate::Result;

#[derive(Copy, Clone)]
pub struct Owned(());

impl crate::traits::Owned for Owned {
    type Reader<'a, A: ReaderArena + 'a> = Reader<'a>;
    type Builder<'a, A: BuilderArena + 'a> = Builder<'a>;
}

pub type Reader<'a> = &'a [u8];

pub(crate) unsafe fn reader_from_raw_parts<'a>(p: *const u8, len: u32) -> Reader<'a> {
    ::core::slice::from_raw_parts(p, len as usize)
}

impl <'a, A> crate::traits::FromPointerReader<'a, A> for Reader<'a> where A: ReaderArena {
    fn get_from_pointer(reader: PointerReader<&'a A>, default: Option<&'a [crate::Word]>) -> Result<Reader<'a>> {
        reader.get_data(default)
    }
}

pub type Builder<'a> = &'a mut [u8];

pub(crate) unsafe fn builder_from_raw_parts<'a>(p: *mut u8, len: u32) -> Builder<'a> {
    ::core::slice::from_raw_parts_mut(p, len as usize)
}

impl <'a, A> crate::traits::FromPointerBuilder<'a, A> for Builder<'a> where A: BuilderArena {
    fn init_pointer(builder: PointerBuilder<&'a mut A>, size : u32) -> Builder<'a> {
        builder.init_data(size)
    }
    fn get_from_pointer(builder: PointerBuilder<&'a mut A>, default: Option<&'a [crate::Word]>) -> Result<Builder<'a>> {
        builder.get_data(default)
    }
}

impl <'a> crate::traits::SetPointerBuilder for Reader<'a> {
    fn set_pointer_builder<'b, B>(pointer: PointerBuilder<&'b mut B>,
                                  value: Reader<'a>,
                                  _canonicalize: bool) -> Result<()>
        where B: BuilderArena
    {
        pointer.set_data(value);
        Ok(())
    }
}

