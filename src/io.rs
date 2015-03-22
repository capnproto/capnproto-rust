// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

//! Input / output.

/// A producer of bytes.
pub trait InputStream {
    /// Reads at least `min_bytes` into `buf` unless EOF is encountered first. Returns the
    /// number of bytes read.
    fn try_read(&mut self, buf : &mut [u8], min_bytes : usize) -> ::std::io::Result<usize>;

    /// Reads at least `min_bytes` into `buf`, returning the number of bytes read. If EOF is
    /// encountered first, returns an error.
    fn read(&mut self, buf : &mut [u8], min_bytes : usize) -> ::std::io::Result<usize> {
        let n = try!(self.try_read(buf, min_bytes));
        if n < min_bytes {
            Err(::std::io::Error::new(::std::io::ErrorKind::Other, "Premature EOF", None))
        } else {
            Ok(n)
        }
    }

    /// Reads into `buf` until it is full. Returns an error if EOF is encountered first.
    fn read_exact(&mut self, buf : &mut [u8]) -> ::std::io::Result<()> {
        let min_bytes = buf.len();
        try!(self.read(buf, min_bytes));
        Ok(())
    }
}

impl <R> InputStream for R where R : ::std::io::Read {
    fn try_read(&mut self, buf : &mut [u8], min_bytes : usize) -> ::std::io::Result<usize> {
        let mut pos = 0;
        while pos < min_bytes {
            let buf1 = &mut buf[pos ..];
            let n = try!(self.read(buf1));
            pos += n;
            if n == 0 { return Ok(pos); }
        }
        return Ok(pos);
    }
}

pub trait BufferedInputStream : InputStream {
    fn skip(&mut self, bytes : usize) -> ::std::io::Result<()>;
    unsafe fn get_read_buffer(&mut self) -> ::std::io::Result<(*const u8, *const u8)>;
}

pub struct BufferedInputStreamWrapper<R> {
    inner : R,
    buf : Vec<u8>,
    pos : usize,
    cap : usize
}

impl <R> BufferedInputStreamWrapper<R> {
    pub fn new(r : R) -> BufferedInputStreamWrapper<R> {
        let mut result = BufferedInputStreamWrapper {
            inner : r,
            buf : Vec::with_capacity(8192),
            pos : 0,
            cap : 0
        };
        unsafe {
            result.buf.set_len(8192)
        }
        return result;
    }
}

impl<R: InputStream> BufferedInputStream for BufferedInputStreamWrapper<R> {

   fn skip(&mut self, mut bytes : usize) -> ::std::io::Result<()> {
        let available = self.cap - self.pos;
        if bytes <= available {
            self.pos += bytes;
        } else {
            bytes -= available;
            if bytes <= self.buf.len() {
                //# Read the next buffer-full.
                let n = try!(self.inner.try_read(self.buf.as_mut_slice(), bytes));
                self.pos = bytes;
                self.cap = n;
            } else {
                //# Forward large skip to the underlying stream.
                panic!("TODO")
            }
        }
        Ok(())
    }

    unsafe fn get_read_buffer(&mut self) -> ::std::io::Result<(*const u8, *const u8)> {
        if self.cap - self.pos == 0 {
            let n = try!(self.inner.try_read(self.buf.as_mut_slice(), 1));
            self.cap = n;
            self.pos = 0;
        }
        Ok((self.buf.get_unchecked(self.pos) as *const u8,
            self.buf.get_unchecked(self.cap) as *const u8))
    }
}

impl<R: InputStream> InputStream for BufferedInputStreamWrapper<R> {
    fn try_read(&mut self, mut dst: &mut [u8], mut min_bytes : usize) -> ::std::io::Result<usize> {
        if min_bytes <= self.cap - self.pos {
            // Serve from the current buffer.
            let n = ::std::cmp::min(self.cap - self.pos, dst.len());
            ::std::slice::bytes::copy_memory(dst,
                                             &self.buf[self.pos .. self.pos + n]);
            self.pos += n;
            return Ok(n);
        } else {
            // Copy current available into destination.
            ::std::slice::bytes::copy_memory(dst,
                                             &self.buf[self.pos .. self.cap]);
            let from_first_buffer = self.cap - self.pos;

            let dst = &mut dst[from_first_buffer ..];
            min_bytes -= from_first_buffer;

            if dst.len() <= self.buf.len() {
                // Read the next buffer-full.
                let n = try!(self.inner.try_read(self.buf.as_mut_slice(), min_bytes));
                let from_second_buffer = ::std::cmp::min(n, dst.len());
                ::std::slice::bytes::copy_memory(dst,
                                                 &self.buf[0 .. from_second_buffer]);
                self.cap = n;
                self.pos = from_second_buffer;
                return Ok(from_first_buffer + from_second_buffer);
            } else {
                // Forward large read to the underlying stream.
                self.pos = 0;
                self.cap = 0;
                return Ok(from_first_buffer + try!(self.inner.try_read(dst, min_bytes)));
            }
        }
    }
}

pub struct ArrayInputStream<'a> {
    array : &'a [u8]
}

impl <'a> ArrayInputStream<'a> {
    pub fn new<'b>(array : &'b [u8]) -> ArrayInputStream<'b> {
        ArrayInputStream { array : array }
    }
}

impl <'a> InputStream for ArrayInputStream<'a> {
    fn try_read(&mut self, dst: &mut [u8], _min_bytes : usize) -> ::std::io::Result<usize> {
        let n = ::std::cmp::min(dst.len(), self.array.len());
        ::std::slice::bytes::copy_memory(dst, &self.array[0 .. n]);
        self.array = &self.array[n ..];
        Ok(n)
    }
}

impl <'a> BufferedInputStream for ArrayInputStream<'a> {
    fn skip(&mut self, bytes : usize) -> ::std::io::Result<()> {
        assert!(self.array.len() >= bytes,
                "ArrayInputStream ended prematurely.");
        self.array = &self.array[bytes ..];
        Ok(())
    }
    unsafe fn get_read_buffer(&mut self) -> ::std::io::Result<(*const u8, *const u8)> {
        let len = self.array.len();
        Ok((self.array.as_ptr() as *const u8,
           self.array.get_unchecked(len) as *const u8))
    }
}

/// A consumer of bytes.
pub trait OutputStream {
    /// Writes all of `buf`.
    fn write(&mut self, buf : &[u8]) -> ::std::io::Result<()>;
    fn flush(&mut self) -> ::std::io::Result<()> { Ok(()) }
}

impl <W> OutputStream for W where W : ::std::io::Write {
    fn write(&mut self, buf : &[u8]) -> ::std::io::Result<()> {
        self.write_all(buf)
    }
    fn flush(&mut self) -> ::std::io::Result<()> {
        self.flush()
    }
}


pub trait BufferedOutputStream : OutputStream {
    unsafe fn get_write_buffer(&mut self) -> (*mut u8, *mut u8);
    unsafe fn write_ptr(&mut self, ptr: *mut u8, size: usize) -> ::std::io::Result<()>;
}

pub struct BufferedOutputStreamWrapper<'a, W:'a> {
    inner: &'a mut W,
    buf: Vec<u8>,
    pos: usize
}

impl <'a, W> BufferedOutputStreamWrapper<'a, W> {
    pub fn new<'b> (w : &'b mut W) -> BufferedOutputStreamWrapper<W> {
        let mut result = BufferedOutputStreamWrapper {
            inner: w,
            buf : Vec::with_capacity(8192),
            pos : 0
        };
        unsafe {
            result.buf.set_len(8192);
        }
        return result;
    }
}

impl<'a, W: OutputStream> BufferedOutputStream for BufferedOutputStreamWrapper<'a, W> {
    #[inline]
    unsafe fn get_write_buffer(&mut self) -> (*mut u8, *mut u8) {
        let len = self.buf.len();
        (self.buf.get_unchecked_mut(self.pos) as *mut u8,
         self.buf.get_unchecked_mut(len) as *mut u8)
    }

    #[inline]
    unsafe fn write_ptr(&mut self, ptr: *mut u8, size: usize) -> ::std::io::Result<()> {
        let easy_case = ptr == self.buf.get_unchecked_mut(self.pos) as *mut u8;
        if easy_case {
            self.pos += size;
            Ok(())
        } else {
            let buf = ::std::slice::from_raw_parts_mut::<u8>(ptr, size);
            self.write(buf)
        }
    }

}

impl<'a, W: OutputStream> OutputStream for BufferedOutputStreamWrapper<'a, W> {
    fn write(&mut self, buf: &[u8]) -> ::std::io::Result<()> {
        let available = self.buf.len() - self.pos;
        let mut size = buf.len();
        if size <= available {
            let dst = &mut self.buf.as_mut_slice()[self.pos ..];
            ::std::slice::bytes::copy_memory(dst, buf);
            self.pos += size;
        } else if size <= self.buf.len() {
            // Too much for this buffer, but not a full buffer's
            // worth, so we'll go ahead and copy.
            {
                let dst = &mut self.buf.as_mut_slice()[self.pos ..];
                ::std::slice::bytes::copy_memory(dst, &buf[0 .. available]);
            }
            try!(self.inner.write(self.buf.as_mut_slice()));

            size -= available;
            let src = &buf[available ..];
            let dst = &mut self.buf.as_mut_slice()[0 ..];
            ::std::slice::bytes::copy_memory(dst, src);
            self.pos = size;
        } else {
            // Writing so much data that we might as well write
            // directly to avoid a copy.
            try!(self.inner.write(&self.buf[0 .. self.pos]));
            self.pos = 0;
            try!(self.inner.write(buf));
        }
        return Ok(());
    }

    fn flush(&mut self) -> ::std::io::Result<()> {
        if self.pos > 0 {
            try!(self.inner.write(&self.buf[0 .. self.pos]));
            self.pos = 0;
        }
        self.inner.flush()
    }
}

pub struct ArrayOutputStream<'a> {
    array : &'a mut [u8],
    fill_pos : usize,
}

impl <'a> ArrayOutputStream<'a> {
    pub fn new<'b>(array : &'b mut [u8]) -> ArrayOutputStream<'b> {
        ArrayOutputStream {
            array : array,
            fill_pos : 0
        }
    }
}

impl <'a> OutputStream for ArrayOutputStream<'a> {
    fn write(&mut self, buf: &[u8]) -> ::std::io::Result<()> {
        assert!(buf.len() <= self.array.len() - self.fill_pos,
                "ArrayOutputStream's backing array was not large enough for the data written.");
        ::std::slice::bytes::copy_memory(&mut self.array[self.fill_pos ..], buf);
        self.fill_pos += buf.len();
        Ok(())
    }
}

impl <'a> BufferedOutputStream for ArrayOutputStream<'a> {
    unsafe fn get_write_buffer(&mut self) -> (*mut u8, *mut u8) {
        let len = self.array.len();
        (self.array.get_unchecked_mut(self.fill_pos) as *mut u8,
         self.array.get_unchecked_mut(len) as *mut u8)
    }
    unsafe fn write_ptr(&mut self, ptr: *mut u8, size: usize) -> ::std::io::Result<()> {
        let easy_case = ptr == self.array.get_unchecked_mut(self.fill_pos) as *mut u8;
        if easy_case {
            self.fill_pos += size;
            Ok(())
        } else {
            let buf = ::std::slice::from_raw_parts_mut::<u8>(ptr, size);
            self.write(buf)
        }
    }
}
