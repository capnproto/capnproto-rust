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

use common::Word;
use private::layout::{StructReader, StructBuilder, StructSize, PointerBuilder, PointerReader};

pub trait FromStructReader<'a> {
    fn new(reader : StructReader<'a>) -> Self;
}

pub trait HasStructSize {
    fn struct_size(unused_self : Option<Self>) -> StructSize;
}

pub trait FromStructBuilder<'a> {
    fn new(structBuilder : StructBuilder<'a>) -> Self;
}

pub trait FromPointerReader<'a> {
    fn get_from_pointer(reader : &PointerReader<'a>) -> Self;
}

pub trait FromPointerReaderRefDefault<'a> {
    fn get_from_pointer(reader : &PointerReader<'a>, default_value : *const Word) -> Self;
}

pub trait FromPointerBuilder<'a> {
    fn init_pointer(PointerBuilder<'a>, u32) -> Self;
    fn get_from_pointer(builder : PointerBuilder<'a>) -> Self;
}

pub trait FromPointerBuilderRefDefault<'a> {
    fn get_from_pointer(builder : PointerBuilder<'a>, default_value : *const Word) -> Self;
}

pub trait SetPointerBuilder<To> {
    fn set_pointer_builder<'a>(PointerBuilder<'a>, Self);
}

pub trait HasTypeId {
    fn type_id(unused_self : Option<Self>) -> u64;
}

pub trait CastableTo<T> {
    fn cast(self) -> T;
}

/// Because `#[derive(ToPrimitive)]` is not supported, using our own custom trait is more
/// convenient than using `ToPrimitive`.
pub trait ToU16 {
    fn to_u16(self) -> u16;
}

