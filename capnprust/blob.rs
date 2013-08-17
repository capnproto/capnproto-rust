/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

pub mod Text {
    use common::*;
    use arena::*;

    pub type Reader<'self> = &'self str;

    pub struct Builder {
        segment : @mut SegmentBuilder,
        ptr : ByteCount,
        elementCount : ElementCount
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

    pub type Reader<'self> = &'self [u8];

    pub struct Builder {
        segment : @mut SegmentBuilder,
        ptr : ByteCount,
        elementCount : ElementCount
    }
}
