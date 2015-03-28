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

use io::{BufferedInputStream, BufferedOutputStream,
         BufferedInputStreamWrapper, BufferedOutputStreamWrapper,
         InputStream, OutputStream};
use message::*;
use serialize;
use Result;

trait PtrUsize<T>: ::std::marker::PhantomFn<T> {
    fn as_usize(self) -> usize;
}

impl <T> PtrUsize<T> for *const T {
    fn as_usize(self) -> usize {
        self as usize
    }
}

impl <T> PtrUsize<T> for *mut T {
    fn as_usize(self) -> usize {
        self as usize
    }
}

#[inline]
fn ptr_sub<T, U: PtrUsize<T>, V: PtrUsize<T>>(p1 : U, p2 : V) -> usize {
    return (p1.as_usize() - p2.as_usize()) / ::std::mem::size_of::<T>();
}

struct PackedInputStream<'a, R : 'a> {
    pub inner : &'a mut R
}

macro_rules! refresh_buffer(
    ($inner:expr, $size:ident, $in_ptr:ident, $in_end:ident, $out:ident,
     $outBuf:ident, $buffer_begin:ident) => (
        {
            try!($inner.skip($size));
            let (b, e) = try!($inner.get_read_buffer());
            $in_ptr = b;
            $in_end = e;
            $size = ptr_sub($in_end, $in_ptr);
            $buffer_begin = b;
            assert!($size > 0);
        }
        );
    );

impl <'a, R : BufferedInputStream> InputStream for PackedInputStream<'a, R> {
    fn try_read(&mut self, out_buf: &mut [u8], _min_bytes : usize) -> ::std::io::Result<usize> {
        let len = out_buf.len();

        if len == 0 { return Ok(0); }

        assert!(len % 8 == 0, "PackedInputStream reads must be word-aligned");

        unsafe {
            let mut out = out_buf.as_mut_ptr();
            let out_end : *mut u8 = out_buf.get_unchecked_mut(len);

            let (mut in_ptr, mut in_end) = try!(self.inner.get_read_buffer());
            let mut buffer_begin = in_ptr;
            let mut size = ptr_sub(in_end, in_ptr);
            if size == 0 {
                return Ok(0);
            }

            loop {

                let mut tag : u8;

                assert!(ptr_sub(out, out_buf.as_mut_ptr()) % 8 == 0,
                        "Output pointer should always be aligned here.");

                if ptr_sub(in_end, in_ptr) < 10 {
                    if out >= out_end {
                        try!(self.inner.skip(ptr_sub(in_ptr, buffer_begin)));
                        return Ok(ptr_sub(out, out_buf.as_mut_ptr()));
                    }

                    if ptr_sub(in_end, in_ptr) == 0 {
                        refresh_buffer!(self.inner, size, in_ptr, in_end, out, out_buf, buffer_begin);
                        continue;
                    }

                    //# We have at least 1, but not 10, bytes available. We need to read
                    //# slowly, doing a bounds check on each byte.

                    tag = *in_ptr;
                    in_ptr = in_ptr.offset(1);

                    for i in 0..8 {
                        if (tag & (1u8 << i)) != 0 {
                            if ptr_sub(in_end, in_ptr) == 0 {
                                refresh_buffer!(self.inner, size, in_ptr, in_end,
                                                out, out_buf, buffer_begin);
                            }
                            *out = *in_ptr;
                            out = out.offset(1);
                            in_ptr = in_ptr.offset(1);
                        } else {
                            *out = 0;
                            out = out.offset(1);
                        }
                    }

                    if ptr_sub(in_end, in_ptr) == 0 && (tag == 0 || tag == 0xff) {
                        refresh_buffer!(self.inner, size, in_ptr, in_end,
                                        out, out_buf, buffer_begin);
                    }
                } else {
                    tag = *in_ptr;
                    in_ptr = in_ptr.offset(1);

                    for n in 0..8 {
                        let is_nonzero = (tag & (1u8 << n)) != 0;
                        *out = (*in_ptr) & ((-(is_nonzero as i8)) as u8);
                        out = out.offset(1);
                        in_ptr = in_ptr.offset(is_nonzero as isize);
                    }
                }
                if tag == 0 {
                    assert!(ptr_sub(in_end, in_ptr) > 0,
                            "Should always have non-empty buffer here");

                    let run_length : usize = (*in_ptr) as usize * 8;
                    in_ptr = in_ptr.offset(1);

                    if run_length > ptr_sub(out_end, out) {
                        return Err(::std::io::Error::new(::std::io::ErrorKind::Other,
                                                         "Packed input did not end cleanly on a segment boundary",
                                                         None));
                    }

                    ::std::ptr::write_bytes(out, 0, run_length);
                    out = out.offset(run_length as isize);

                } else if tag == 0xff {
                    assert!(ptr_sub(in_end, in_ptr) > 0,
                            "Should always have non-empty buffer here");

                    let mut run_length : usize = (*in_ptr) as usize * 8;
                    in_ptr = in_ptr.offset(1);

                    if run_length > ptr_sub(out_end, out) {
                        return Err(::std::io::Error::new(::std::io::ErrorKind::Other,
                                                         "Packed input did not end cleanly on a segment boundary",
                                                         None));
                    }

                    let in_remaining = ptr_sub(in_end, in_ptr);
                    if in_remaining >= run_length {
                        //# Fast path.
                        ::std::ptr::copy_nonoverlapping(out, in_ptr, run_length);
                        out = out.offset(run_length as isize);
                        in_ptr = in_ptr.offset(run_length as isize);
                    } else {
                        //# Copy over the first buffer, then do one big read for the rest.
                        ::std::ptr::copy_nonoverlapping(out, in_ptr, in_remaining);
                        out = out.offset(in_remaining as isize);
                        run_length -= in_remaining;

                        try!(self.inner.skip(size));
                        {
                            let buf = ::std::slice::from_raw_parts_mut::<u8>(out, run_length);
                            try!(self.inner.read_exact(buf));
                        }

                        out = out.offset(run_length as isize);

                        if out == out_end {
                            return Ok(len);
                        } else {
                            let (b, e) = try!(self.inner.get_read_buffer());
                            in_ptr = b;
                            in_end = e;
                            size = ptr_sub(e, b);
                            buffer_begin = in_ptr;
                            continue;
                        }
                    }
                }

                if out == out_end {
                    try!(self.inner.skip(ptr_sub(in_ptr, buffer_begin)));
                    return Ok(len);
                }
            }
        }
    }
}



pub fn new_reader<U : BufferedInputStream>(input : &mut U,
                                           options : ReaderOptions)
                                           -> Result<serialize::OwnedSpaceMessageReader> {
    let mut packed_input = PackedInputStream {
        inner : input
    };

    serialize::new_reader(&mut packed_input, options)
}

pub fn new_reader_unbuffered<U : InputStream>(input : U,
                                              options : ReaderOptions)
                                              -> Result<serialize::OwnedSpaceMessageReader> {
    let mut packed_input = PackedInputStream {
        inner : &mut BufferedInputStreamWrapper::new(input)
    };

    serialize::new_reader(&mut packed_input, options)
}


struct PackedOutputStream<'a, W:'a> {
    pub inner : &'a mut W
}

impl <'a, W : BufferedOutputStream> OutputStream for PackedOutputStream<'a, W> {
    fn write(&mut self, in_buf : &[u8]) -> ::std::io::Result<()> {
        unsafe {
            let (mut out, mut buffer_end) = self.inner.get_write_buffer();
            let mut buffer_begin = out;
            let mut slow_buffer : [u8; 20] = [0; 20];

            let mut in_ptr : *const u8 = in_buf.get_unchecked(0);
            let in_end : *const u8 = in_buf.get_unchecked(in_buf.len());

            while in_ptr < in_end {

                if ptr_sub(buffer_end, out) < 10 {
                    //# Oops, we're out of space. We need at least 10
                    //# bytes for the fast path, since we don't
                    //# bounds-check on every byte.
                    try!(self.inner.write_ptr(buffer_begin, ptr_sub(out, buffer_begin)));

                    out = slow_buffer.as_mut_ptr();
                    buffer_end = slow_buffer.get_unchecked_mut(20);
                    buffer_begin = out;
                }

                let tag_pos : *mut u8 = out;
                out = out.offset(1);

                let bit0 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit0 as isize);
                in_ptr = in_ptr.offset(1);

                let bit1 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit1 as isize);
                in_ptr = in_ptr.offset(1);

                let bit2 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit2 as isize);
                in_ptr = in_ptr.offset(1);

                let bit3 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit3 as isize);
                in_ptr = in_ptr.offset(1);

                let bit4 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit4 as isize);
                in_ptr = in_ptr.offset(1);

                let bit5 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit5 as isize);
                in_ptr = in_ptr.offset(1);

                let bit6 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit6 as isize);
                in_ptr = in_ptr.offset(1);

                let bit7 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit7 as isize);
                in_ptr = in_ptr.offset(1);


                let tag : u8 = (bit0 << 0) | (bit1 << 1) | (bit2 << 2) | (bit3 << 3)
                    | (bit4 << 4) | (bit5 << 5) | (bit6 << 6) | (bit7 << 7);

                *tag_pos = tag;

                if tag == 0 {
                    //# An all-zero word is followed by a count of
                    //# consecutive zero words (not including the first
                    //# one).

                    let mut in_word : *const u64 = ::std::mem::transmute(in_ptr);
                    let mut limit : *const u64 = ::std::mem::transmute(in_end);
                    if ptr_sub(limit, in_word) > 255 {
                        limit = in_word.offset(255);
                    }
                    while in_word < limit && *in_word == 0 {
                        in_word = in_word.offset(1);
                    }
                    *out = ptr_sub(in_word, ::std::mem::transmute::<*const u8, *const u64>(in_ptr)) as u8;

                    out = out.offset(1);
                    in_ptr = ::std::mem::transmute::<*const u64, *const u8>(in_word);

                } else if tag == 0xff {
                    //# An all-nonzero word is followed by a count of
                    //# consecutive uncompressed words, followed by the
                    //# uncompressed words themselves.

                    //# Count the number of consecutive words in the input
                    //# which have no more than a single zero-byte. We look
                    //# for at least two zeros because that's the point
                    //# where our compression scheme becomes a net win.
                    let run_start = in_ptr;
                    let mut limit = in_end;
                    if ptr_sub(limit, in_ptr) > 255 * 8 {
                        limit = in_ptr.offset(255 * 8);
                    }

                    while in_ptr < limit {
                        let mut c = 0;

                        for _ in 0..8 {
                            c += (*in_ptr == 0) as u8;
                            in_ptr = in_ptr.offset(1);
                        }

                        if c >= 2 {
                            //# Un-read the word with multiple zeros, since
                            //# we'll want to compress that one.
                            in_ptr = in_ptr.offset(-8);
                            break;
                        }
                    }
                    let count : usize = ptr_sub(in_ptr, run_start);
                    *out = (count / 8) as u8;
                    out = out.offset(1);

                    if count <= ptr_sub(buffer_end, out) {
                        //# There's enough space to memcpy.

                        let src : *const u8 = run_start;
                        ::std::ptr::copy_nonoverlapping(out, src, count);

                        out = out.offset(count as isize);
                    } else {
                        //# Input overruns the output buffer. We'll give it
                        //# to the output stream in one chunk and let it
                        //# decide what to do.
                        try!(self.inner.write_ptr(buffer_begin, ptr_sub(out, buffer_begin)));

                        {
                            let buf = ::std::slice::from_raw_parts::<u8>(run_start, count);
                            try!(self.inner.write(buf));
                        }

                        let (out1, buffer_end1) = self.inner.get_write_buffer();
                        out = out1; buffer_end = buffer_end1;
                        buffer_begin = out;
                    }
                }
            }

            try!(self.inner.write_ptr(buffer_begin, ptr_sub(out, buffer_begin)));
            Ok(())
        }
    }

   fn flush(&mut self) -> ::std::io::Result<()> { self.inner.flush() }
}

pub fn write_packed_message<T: BufferedOutputStream, U: MessageBuilder>(
    output : &mut T, message : &mut U) -> ::std::io::Result<()> {
    let mut packed_output_stream = PackedOutputStream {inner : output};
    serialize::write_message(&mut packed_output_stream, message)
}


pub fn write_packed_message_unbuffered<T: OutputStream, U: MessageBuilder>(
    output : &mut T, message : &mut U) -> ::std::io::Result<()> {
    let mut buffered = BufferedOutputStreamWrapper::new(output);
    try!(write_packed_message(&mut buffered, message));
    buffered.flush()
}

#[cfg(test)]
mod tests {
    use std;
    use serialize_packed::{PackedOutputStream, PackedInputStream};
    use io::{ArrayInputStream, ArrayOutputStream, InputStream, OutputStream};

    pub fn expect_packs_to(unpacked : &[u8],
                           packed : &[u8]) {

        // --------
        // write

        let mut bytes : std::vec::Vec<u8> = ::std::iter::repeat(0u8).take(packed.len()).collect();
        {
            let mut writer = ArrayOutputStream::new(&mut bytes[..]);
            let mut packed_output_stream = PackedOutputStream {inner : &mut writer};
            packed_output_stream.write(unpacked).unwrap();
        }

        assert_eq!(bytes, packed);

        // --------
        // read

        let mut reader = ArrayInputStream::new(packed);
        let mut packed_input_stream = PackedInputStream {inner : &mut reader};


        let mut bytes : std::vec::Vec<u8> = ::std::iter::repeat(0u8).take(unpacked.len()).collect();
        packed_input_stream.read_exact(&mut bytes[..]).unwrap();

        //    assert!(packed_input_stream.eof());
        assert_eq!(bytes, unpacked);
    }

    static ZEROES : &'static[u8] = &[0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0];

    #[test]
    pub fn simple_packing() {
        expect_packs_to(&[], &[]);
        expect_packs_to(&ZEROES[0 .. 8], &[0,0]);
        expect_packs_to(&[0,0,12,0,0,34,0,0], &[0x24,12,34]);
        expect_packs_to(&[1,3,2,4,5,7,6,8], &[0xff,1,3,2,4,5,7,6,8,0]);
        expect_packs_to(&[0,0,0,0,0,0,0,0,1,3,2,4,5,7,6,8], &[0,0,0xff,1,3,2,4,5,7,6,8,0]);
        expect_packs_to(&[0,0,12,0,0,34,0,0,1,3,2,4,5,7,6,8], &[0x24,12,34,0xff,1,3,2,4,5,7,6,8,0]);
        expect_packs_to(&[1,3,2,4,5,7,6,8,8,6,7,4,5,2,3,1], &[0xff,1,3,2,4,5,7,6,8,1,8,6,7,4,5,2,3,1]);

        expect_packs_to(
            &[1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 0,2,4,0,9,0,5,1],
            &[0xff,1,2,3,4,5,6,7,8, 3, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8,
              0xd6,2,4,9,5,1]);
        expect_packs_to(
            &[1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 6,2,4,3,9,0,5,1, 1,2,3,4,5,6,7,8, 0,2,4,0,9,0,5,1],
            &[0xff,1,2,3,4,5,6,7,8, 3, 1,2,3,4,5,6,7,8, 6,2,4,3,9,0,5,1, 1,2,3,4,5,6,7,8,
              0xd6,2,4,9,5,1]);

        expect_packs_to(
            &[8,0,100,6,0,1,1,2, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,1,0,2,0,3,1],
            &[0xed,8,100,6,1,1,2, 0,2, 0xd4,1,2,3,1]);

        expect_packs_to(&ZEROES[0 .. 16], &[0,1]);
        expect_packs_to(&[0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0], &[0,2]);

    }
}
