/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use std::io::{Reader, Writer};


pub fn read_at_least<R : Reader>(reader : &mut R,
                                 buf: &mut [u8],
                                 min_bytes : uint) -> uint {
    let mut pos = 0;
    let bufLen = buf.len();
    while pos < min_bytes {
        let buf1 = buf.mut_slice(pos, bufLen);
        match reader.read(buf1) {
            None => fail!("premature EOF?"),
            Some(n) => pos += n
        }
    }
    return pos;
}

pub struct BufferedInputStream<'a, R> {
    priv inner : &'a mut R,
    priv buf : ~[u8],
    priv pos : uint,
    priv cap : uint
}

impl<'a, R: Reader> BufferedInputStream<'a, R> {
    pub fn new<'a> (r : &'a mut R) -> BufferedInputStream<'a, R> {
        let mut result = BufferedInputStream {
            inner : r,
            buf : std::vec::with_capacity(8192),
            pos : 0,
            cap : 0
        };
        unsafe {
            std::vec::raw::set_len(&mut result.buf, 8192)
        }
        return result;
    }

    pub fn skip(&mut self, mut bytes : uint) {
        let available = self.cap - self.pos;
        if bytes <= available {
            self.pos += bytes;
        } else {
            bytes -= available;
            if (bytes <= self.buf.len()) {
                //# Read the next buffer-full.
                let n = read_at_least(self.inner, self.buf, bytes);
                self.pos = bytes;
                self.cap = n;

            } else {
                //# Forward large skip to the underlying stream.
                fail!("TODO")
            }
        }
    }

    pub unsafe fn getReadBuffer(&mut self) -> (*u8, *u8) {
        if self.cap - self.pos == 0 {
            let n = read_at_least(self.inner, self.buf, 1);
            self.cap = n;
            self.pos = 0;
        }
        (self.buf.unsafe_ref(self.pos), self.buf.unsafe_ref(self.cap))
    }
}

impl<'a, R: Reader> Reader for BufferedInputStream<'a, R> {
    fn eof(&mut self) -> bool {
        self.inner.eof()
    }

    fn read(&mut self, dst: &mut [u8]) -> Option<uint> {
        let mut num_bytes = dst.len();
        if (num_bytes <= self.cap - self.pos) {
            //# Serve from the current buffer.
            std::vec::bytes::copy_memory(dst,
                                         self.buf.slice(self.pos, self.cap),
                                         num_bytes);
            self.pos += num_bytes;
            return Some(num_bytes);
        } else {
            //# Copy current available into destination.

            std::vec::bytes::copy_memory(dst,
                                         self.buf.slice(self.pos, self.cap),
                                         self.cap - self.pos);
            let fromFirstBuffer = self.cap - self.pos;

            let dst1 = dst.mut_slice(fromFirstBuffer, num_bytes);
            num_bytes -= fromFirstBuffer;
            if (num_bytes <= self.buf.len()) {
                //# Read the next buffer-full.
                let n = read_at_least(self.inner, self.buf, num_bytes);
                std::vec::bytes::copy_memory(dst1,
                                             self.buf.slice(0, num_bytes),
                                             num_bytes);
                self.cap = n;
                self.pos = num_bytes;
                return Some(fromFirstBuffer + num_bytes);
            } else {
                //# Forward large read to the underlying stream.
                self.pos = 0;
                self.cap = 0;
                return Some(fromFirstBuffer + read_at_least(self.inner, dst1, num_bytes));
            }
        }
    }
}

pub struct BufferedOutputStream<'a, W> {
    priv inner: &'a mut W,
    priv buf: ~[u8],
    priv pos: uint
}

impl<'a, W: Writer> BufferedOutputStream<'a, W> {

    pub fn new<'b> (w : &'b mut W) -> BufferedOutputStream<'b, W> {
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
    pub unsafe fn getWriteBuffer(&mut self) -> (*mut u8, *mut u8) {
        let len = self.buf.len();
        (self.buf.unsafe_mut_ref(self.pos), self.buf.unsafe_mut_ref(len))
    }

    #[inline]
    pub unsafe fn write_ptr(&mut self, ptr: *mut u8, size: uint) {
        let easyCase = ptr == self.buf.unsafe_mut_ref(self.pos);
        if easyCase {
            self.pos += size;
        } else {
            std::vec::raw::mut_buf_as_slice::<u8,()>(ptr, size, |buf| {
                self.write(buf);
            })
        }
    }
}


impl<'a, W: Writer> Writer for BufferedOutputStream<'a, W> {
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
