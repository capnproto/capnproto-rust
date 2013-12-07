/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

pub mod AnyPointer {
    use layout::{PointerReader, PointerBuilder};

    pub struct Reader<'a> {
        reader : PointerReader<'a>
    }

    pub struct Builder {
        builder : PointerBuilder
    }
}
