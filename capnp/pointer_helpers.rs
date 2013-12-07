/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use common::Word;
use layout::{PointerReader, PointerBuilder, FromStructReader};
//use list::{PrimitiveList};

pub trait PointerReaderHelpers<'a> {
    fn get(reader : PointerReader<'a>, default_value : *Word) -> Self;
}


// Hmm. I think we need associated types to do this right.
pub trait PointerBuilderHelpers<R> {
    fn get(builder : PointerBuilder, default_value : *Word) -> Self;
    fn set(builder : PointerBuilder, value : R);
    fn init(builder : PointerBuilder) -> Self;
}

impl <'a, T: FromStructReader<'a>> PointerReaderHelpers<'a> for T {
    fn get(reader : PointerReader<'a>, default_value : *Word) -> T {
        FromStructReader::from_struct_reader(reader.get_struct(default_value))
    }
}

/*
Can't do this because of "conflicting implementations"

impl <'a, T: PrimitiveElement> PointerReaderHelpers<'a> for PrimitiveList::Reader<'a, T> {
    fn get(reader : PointerReader<'a>, default_value : *Word) -> PrimitiveList::Reader<'a, T> {
        fail!();
//        reader.get_list(f
    }
}
*/
