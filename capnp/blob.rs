/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

pub mod Text {
    use common::*;
    use arena::*;

    pub type Reader<'a> = &'a str;

    pub struct Builder<'a> {
        segment : *mut SegmentBuilder<'a>,
        ptr : ByteCount,
        //...
    }

/*
    impl Builder {
        pub fn asStr(&self) -> ...
    }
*/
}

pub mod Data {
    use common::*;
    use arena::*;

    pub type Reader<'a> = &'a [u8];

    pub struct Builder<'a> {
        segment : *mut SegmentBuilder<'a>,
        ptr : ByteCount,
        elementCount : ElementCount
    }
}
