/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use common::Word;
use layout::{StructReader, StructBuilder, StructSize, PointerBuilder, PointerReader};

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

/// Because `#[deriving(ToPrimitive)]` is not supported, using our own custom trait is more
/// convenient than using `ToPrimitive`.
pub trait ToU16 {
    fn to_u16(self) -> u16;
}

