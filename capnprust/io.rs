/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use std::rt::io::Writer;
use common;

pub struct BufferedOutputStream<'self, W> {
    priv inner: &'self mut W,
    priv buf: ~[u8],
    priv pos: uint
}

impl<'self, W: Writer> BufferedOutputStream<'self, W> {

    pub fn new<'a> (w : &'a mut W) -> BufferedOutputStream<'a, W> {
        let mut result = BufferedOutputStream {
            inner: w,
            buf : std::vec::with_capacity(8192),
            pos : 0
        };
        unsafe {
            std::vec::raw::set_len(&mut result.buf, 8192);
        }
        return result;
    }

    #[inline]
    pub fn getWriteBuffer(&mut self) -> (*mut u8, uint) {
        unsafe {
            (self.buf.unsafe_mut_ref(self.pos), self.buf.len() - self.pos)
        }
    }

    #[inline]
    pub fn advance(&mut self, n : uint) {
        self.pos += n;
        assert!(self.pos < self.buf.len());
    }
}


impl<'self, W: Writer> Writer for BufferedOutputStream<'self, W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) {
        let available = self.buf.len() - self.pos;
        let mut size = buf.len();
        if size <= available {
            let dst = self.buf.mut_slice_from(self.pos);
            std::vec::bytes::copy_memory(dst, buf, buf.len());
            self.pos += size;
        } else if size <= self.buf.len() {
            //# Too much for this buffer, but not a full buffer's
            //# worth, so we'll go ahead and copy.
            {
                let dst = self.buf.mut_slice_from(self.pos);
                std::vec::bytes::copy_memory(dst, buf, available);
            }
            self.inner.write(self.buf);

            size -= available;
            let src = buf.slice(available, buf.len());
            let dst = self.buf.mut_slice_from(0);
            std::vec::bytes::copy_memory(dst, src, size);
            self.pos = size;
        } else {
            //# Writing so much data that we might as well write
            //# directly to avoid a copy.
            self.inner.write(self.buf.slice(0, self.pos));
            self.pos = 0;
            self.inner.write(buf);
        }
    }

    fn flush(&mut self) {
        if (self.pos > 0) {
            self.inner.write(self.buf.slice(0, self.pos));
            self.pos = 0;
        }
    }
}
