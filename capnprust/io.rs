/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std::rt::io::Writer;

pub struct BufferedOutputStream<W> {
    priv inner: W,
    priv buf: ~[u8],
    priv pos: uint
}

impl<W: Writer> BufferedOutputStream<W> {
    pub fn getWriteBuffer<'a>(&'a mut self) -> &'a mut [u8] {
        fail!("unimplemented")
    }
}

impl<W: Writer> Writer for BufferedOutputStream<W> {
    fn write(&mut self, _buf: &[u8]) {
        // like BufferedWriter, but first check if buf is a prefix of our inner buf.
        fail!("unimplemented");
    }

    fn flush(&mut self) {
        fail!("unimplemented")
    }
}
