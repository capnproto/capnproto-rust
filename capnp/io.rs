/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
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

pub trait BufferedInputStream : Reader {
    fn skip(&mut self, bytes : uint);
    unsafe fn get_read_buffer(&mut self) -> (*u8, *u8);
}

pub struct BufferedInputStreamWrapper<'a, R> {
    priv inner : &'a mut R,
    priv buf : ~[u8],
    priv pos : uint,
    priv cap : uint
}

impl <'a, R> BufferedInputStreamWrapper<'a, R> {
    pub fn new<'a> (r : &'a mut R) -> BufferedInputStreamWrapper<'a, R> {
        let mut result = BufferedInputStreamWrapper {
            inner : r,
            buf : std::vec::with_capacity(8192),
            pos : 0,
            cap : 0
        };
        unsafe {
            result.buf.set_len(8192)
        }
        return result;
    }
}

impl<'a, R: Reader> BufferedInputStream for BufferedInputStreamWrapper<'a, R> {

   fn skip(&mut self, mut bytes : uint) {
        let available = self.cap - self.pos;
        if bytes <= available {
            self.pos += bytes;
        } else {
            bytes -= available;
            if bytes <= self.buf.len() {
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

    unsafe fn get_read_buffer(&mut self) -> (*u8, *u8) {
        if self.cap - self.pos == 0 {
            let n = read_at_least(self.inner, self.buf, 1);
            self.cap = n;
            self.pos = 0;
        }
        (self.buf.unsafe_ref(self.pos), self.buf.unsafe_ref(self.cap))
    }
}

impl<'a, R: Reader> Reader for BufferedInputStreamWrapper<'a, R> {
    fn read(&mut self, dst: &mut [u8]) -> Option<uint> {
        let mut num_bytes = dst.len();
        if num_bytes <= self.cap - self.pos {
            //# Serve from the current buffer.
            std::vec::bytes::copy_memory(dst,
                                         self.buf.slice(self.pos, self.pos + num_bytes));
            self.pos += num_bytes;
            return Some(num_bytes);
        } else {
            //# Copy current available into destination.

            std::vec::bytes::copy_memory(dst,
                                         self.buf.slice(self.pos, self.cap));
            let fromFirstBuffer = self.cap - self.pos;

            let dst1 = dst.mut_slice(fromFirstBuffer, num_bytes);
            num_bytes -= fromFirstBuffer;
            if num_bytes <= self.buf.len() {
                //# Read the next buffer-full.
                let n = read_at_least(self.inner, self.buf, num_bytes);
                std::vec::bytes::copy_memory(dst1,
                                             self.buf.slice(0, num_bytes));
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

pub struct ArrayInputStream<'a> {
    priv array : &'a [u8]
}

impl <'a> ArrayInputStream<'a> {
    pub fn new<'b>(array : &'b [u8]) -> ArrayInputStream<'b> {
        ArrayInputStream { array : array }
    }
}

impl <'a> Reader for ArrayInputStream<'a> {
    fn read(&mut self, dst: &mut [u8]) -> Option<uint> {
        let n = std::cmp::min(dst.len(), self.array.len());
        unsafe { dst.copy_memory(self.array.slice_to(n)); }
        self.array = self.array.slice_from(n);
        Some(n)
    }
}

impl <'a> BufferedInputStream for ArrayInputStream<'a> {
    fn skip(&mut self, bytes : uint) {
        assert!(self.array.len() >= bytes,
                "ArrayInputStream ended prematurely.");
        self.array = self.array.slice_from(bytes);
    }
    unsafe fn get_read_buffer(&mut self) -> (*u8, *u8){
        let len = self.array.len();
        (self.array.as_ptr(), self.array.unsafe_ref(len))
    }
}

pub trait BufferedOutputStream : Writer {
    unsafe fn get_write_buffer(&mut self) -> (*mut u8, *mut u8);
    unsafe fn write_ptr(&mut self, ptr: *mut u8, size: uint);
}

pub struct BufferedOutputStreamWrapper<'a, W> {
    priv inner: &'a mut W,
    priv buf: ~[u8],
    priv pos: uint
}

impl <'a, W> BufferedOutputStreamWrapper<'a, W> {
    pub fn new<'b> (w : &'b mut W) -> BufferedOutputStreamWrapper<'b, W> {
        let mut result = BufferedOutputStreamWrapper {
            inner: w,
            buf : std::vec::with_capacity(8192),
            pos : 0
        };
        unsafe {
            result.buf.set_len(8192);
        }
        return result;
    }
}

impl<'a, W: Writer> BufferedOutputStream for BufferedOutputStreamWrapper<'a, W> {
    #[inline]
    unsafe fn get_write_buffer(&mut self) -> (*mut u8, *mut u8) {
        let len = self.buf.len();
        (self.buf.unsafe_mut_ref(self.pos), self.buf.unsafe_mut_ref(len))
    }

    #[inline]
    unsafe fn write_ptr(&mut self, ptr: *mut u8, size: uint) {
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


impl<'a, W: Writer> Writer for BufferedOutputStreamWrapper<'a, W> {
    fn write(&mut self, buf: &[u8]) {
        let available = self.buf.len() - self.pos;
        let mut size = buf.len();
        if size <= available {
            let dst = self.buf.mut_slice_from(self.pos);
            std::vec::bytes::copy_memory(dst, buf);
            self.pos += size;
        } else if size <= self.buf.len() {
            //# Too much for this buffer, but not a full buffer's
            //# worth, so we'll go ahead and copy.
            {
                let dst = self.buf.mut_slice_from(self.pos);
                std::vec::bytes::copy_memory(dst, buf.slice(0, available));
            }
            self.inner.write(self.buf);

            size -= available;
            let src = buf.slice_from(available);
            let dst = self.buf.mut_slice_from(0);
            std::vec::bytes::copy_memory(dst, src);
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
        if self.pos > 0 {
            self.inner.write(self.buf.slice(0, self.pos));
            self.pos = 0;
        }
    }
}

pub struct ArrayOutputStream<'a> {
    priv array : &'a mut [u8],
    priv fill_pos : uint,
}

impl <'a> ArrayOutputStream<'a> {
    pub fn new<'b>(array : &'b mut [u8]) -> ArrayOutputStream<'b> {
        ArrayOutputStream {
            array : array,
            fill_pos : 0
        }
    }
}

impl <'a> Writer for ArrayOutputStream<'a> {
    fn write(&mut self, buf: &[u8]) {
        assert!(buf.len() <= self.array.len() - self.fill_pos,
                "ArrayOutputStream's backing array was not large enough for the data written.");
        unsafe { self.array.mut_slice_from(self.fill_pos).copy_memory(buf); }
        self.fill_pos += buf.len();
    }
}

impl <'a> BufferedOutputStream for ArrayOutputStream<'a> {
    unsafe fn get_write_buffer(&mut self) -> (*mut u8, *mut u8) {
        let len = self.array.len();
        (self.array.unsafe_mut_ref(self.fill_pos), self.array.unsafe_mut_ref(len))
    }
    unsafe fn write_ptr(&mut self, ptr: *mut u8, size: uint) {
        let easyCase = ptr == self.array.unsafe_mut_ref(self.fill_pos);
        if easyCase {
            self.fill_pos += size;
        } else {
            std::vec::raw::mut_buf_as_slice::<u8,()>(ptr, size, |buf| {
                self.write(buf);
            })
        }
    }
}
