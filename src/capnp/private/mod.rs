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

//! This `private` module contains implementation details that should never be used by clients. We
//! still need to make it public so that generated code can use it.

use ::common::Word;

pub mod arena;
pub mod endian;
pub mod layout;
mod mask;

#[cfg(test)]
mod layout_test;

/// Some data that's guaranteed to be aligned on a word boundary. Typically
/// the type parameter `T` will be instantiated as `[u8; n]` where `n` is
/// a multiple of eight.
///
/// Perhaps in the future Rust will provide a nicer way to guarantee alignment.
#[repr(C)]
pub struct AlignedData<T> {
    pub _dummy : u64,
    pub data : T
}

pub struct RawSchema<'a> {
    pub id : u64,
    pub blob: &'a [u8], // must be aligned to a word boundary
}

impl <'a> RawSchema<'a> {
    pub fn get_encoded_node(&self) -> &'a [Word] {
        unsafe {
            ::std::slice::from_raw_parts(
                ::std::mem::transmute(&self.blob.as_ptr()),
                self.blob.len() / 8)
        }
    }
}


