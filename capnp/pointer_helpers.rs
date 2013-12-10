/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use common::Word;
use layout::{PointerReader, PointerBuilder, FromStructReader, PrimitiveElement};
use list::{PrimitiveList, FromPointerReader};
use blob::{Text};

pub trait PointerReaderHelpers<'a> {
    fn get(reader : PointerReader<'a>, default_value : *Word) -> Self;
}


// Hmm. I think we may need associated types to do this right.
pub trait PointerBuilderHelpers<R> {
    fn get(builder : PointerBuilder, default_value : *Word) -> Self;
    fn set(builder : PointerBuilder, value : R);
    fn init(builder : PointerBuilder) -> Self;
}


struct FromStructReaderWrapper<T> {
    unwrap : T
}


impl <'a, T: FromStructReader<'a>> PointerReaderHelpers<'a> for FromStructReaderWrapper<T> {
    fn get(reader : PointerReader<'a>, default_value : *Word) -> FromStructReaderWrapper<T> {
        FromStructReaderWrapper {
            unwrap : FromStructReader::from_struct_reader(reader.get_struct(default_value))
        }
    }
}

impl <'a, T: PrimitiveElement> PointerReaderHelpers<'a> for PrimitiveList::Reader<'a, T> {
    fn get(reader : PointerReader<'a>, default_value : *Word) -> PrimitiveList::Reader<'a, T> {
        FromPointerReader::get_from_pointer(&reader, default_value)
    }
}

impl <'a> PointerReaderHelpers<'a> for Text::Reader<'a> {
    fn get(_reader : PointerReader<'a>, _default_value : *Word) -> Text::Reader<'a> {
        fail!()
//        reader.get_text(default_value);
    }
}
