/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use io;
use message::*;
use serialize;
use common::ptr_sub;


pub struct PackedInputStream<'a, R:'a> {
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

impl <'a, R : io::BufferedInputStream> std::io::Reader for PackedInputStream<'a, R> {
    fn read(&mut self, out_buf: &mut [u8]) -> std::io::IoResult<uint> {
        let len = out_buf.len();

        if len == 0 { return Ok(0); }

        assert!(len % 8 == 0, "PackedInputStream reads must be word-aligned");

        unsafe {
            let mut out = out_buf.as_mut_ptr();
            let out_end = out_buf.get_unchecked_mut(len) as *mut u8;

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

                    for i in range(0u, 8) {
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

                    for n in range(0u, 8) {
                        let is_nonzero = (tag & ((1 as u8) << n)) != 0;
                        *out = (*in_ptr) & ((-(is_nonzero as i8)) as u8);
                        out = out.offset(1);
                        in_ptr = in_ptr.offset(is_nonzero as int);
                    }
                }
                if tag == 0 {
                    assert!(ptr_sub(in_end, in_ptr) > 0,
                            "Should always have non-empty buffer here");

                    let run_length : uint = (*in_ptr) as uint * 8;
                    in_ptr = in_ptr.offset(1);

                    assert!(run_length <= ptr_sub(out_end, out),
                            "Packed input did not end cleanly on a segment boundary");

                    std::ptr::set_memory(out, 0, run_length);
                    out = out.offset(run_length as int);

                } else if tag == 0xff {
                    assert!(ptr_sub(in_end, in_ptr) > 0,
                            "Should always have non-empty buffer here");

                    let mut run_length : uint = (*in_ptr) as uint * 8;
                    in_ptr = in_ptr.offset(1);

                    assert!(run_length <= ptr_sub(out_end, out),
                            "Packed input did not end cleanly on a segment boundary");

                    let in_remaining = ptr_sub(in_end, in_ptr);
                    if in_remaining >= run_length {
                        //# Fast path.
                        std::ptr::copy_nonoverlapping_memory(out, in_ptr, run_length);
                        out = out.offset(run_length as int);
                        in_ptr = in_ptr.offset(run_length as int);
                    } else {
                        //# Copy over the first buffer, then do one big read for the rest.
                        std::ptr::copy_nonoverlapping_memory(out, in_ptr, in_remaining);
                        out = out.offset(in_remaining as int);
                        run_length -= in_remaining;

                        try!(self.inner.skip(size));
                        {
                            let buf = std::slice::from_raw_mut_buf::<u8>(&out, run_length);
                            try!(self.inner.read(buf));
                        }

                        out = out.offset(run_length as int);

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



pub fn new_reader<U : io::BufferedInputStream>(input : &mut U,
                                               options : ReaderOptions)
                                               -> std::io::IoResult<serialize::OwnedSpaceMessageReader> {
    let mut packed_input = PackedInputStream {
        inner : input
    };

    serialize::new_reader(&mut packed_input, options)
}

pub fn new_reader_unbuffered<U : std::io::Reader>(input : &mut U,
                                                  options : ReaderOptions)
                                                  -> std::io::IoResult<serialize::OwnedSpaceMessageReader> {
    let mut packed_input = PackedInputStream {
        inner : &mut io::BufferedInputStreamWrapper::new(input)
    };

    serialize::new_reader(&mut packed_input, options)
}


pub struct PackedOutputStream<'a, W:'a> {
    pub inner : &'a mut W
}

impl <'a, W : io::BufferedOutputStream> std::io::Writer for PackedOutputStream<'a, W> {
    fn write(&mut self, in_buf : &[u8]) -> std::io::IoResult<()> {
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
                    buffer_end = slow_buffer.get_unchecked_mut(20) as *mut u8;
                    buffer_begin = out;
                }

                let tag_pos : *mut u8 = out;
                out = out.offset(1);

                let bit0 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit0 as int);
                in_ptr = in_ptr.offset(1);

                let bit1 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit1 as int);
                in_ptr = in_ptr.offset(1);

                let bit2 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit2 as int);
                in_ptr = in_ptr.offset(1);

                let bit3 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit3 as int);
                in_ptr = in_ptr.offset(1);

                let bit4 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit4 as int);
                in_ptr = in_ptr.offset(1);

                let bit5 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit5 as int);
                in_ptr = in_ptr.offset(1);

                let bit6 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit6 as int);
                in_ptr = in_ptr.offset(1);

                let bit7 = (*in_ptr != 0) as u8;
                *out = *in_ptr;
                out = out.offset(bit7 as int);
                in_ptr = in_ptr.offset(1);


                let tag : u8 = (bit0 << 0) | (bit1 << 1) | (bit2 << 2) | (bit3 << 3)
                    | (bit4 << 4) | (bit5 << 5) | (bit6 << 6) | (bit7 << 7);

                *tag_pos = tag;

                if tag == 0 {
                    //# An all-zero word is followed by a count of
                    //# consecutive zero words (not including the first
                    //# one).

                    let mut in_word : *const u64 = std::mem::transmute(in_ptr);
                    let mut limit : *const u64 = std::mem::transmute(in_end);
                    if ptr_sub(limit, in_word) > 255 {
                        limit = in_word.offset(255);
                    }
                    while in_word < limit && *in_word == 0 {
                        in_word = in_word.offset(1);
                    }
                    *out = ptr_sub(in_word, std::mem::transmute::<*const u8, *const u64>(in_ptr)) as u8;

                    out = out.offset(1);
                    in_ptr = std::mem::transmute::<*const u64, *const u8>(in_word);

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

                        for _ in range(0u, 8) {
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
                    let count : uint = ptr_sub(in_ptr, run_start);
                    *out = (count / 8) as u8;
                    out = out.offset(1);

                    if count <= ptr_sub(buffer_end, out) {
                        //# There's enough space to memcpy.

                        let src : *const u8 = run_start;
                        std::ptr::copy_nonoverlapping_memory(out, src, count);

                        out = out.offset(count as int);
                    } else {
                        //# Input overruns the output buffer. We'll give it
                        //# to the output stream in one chunk and let it
                        //# decide what to do.
                        try!(self.inner.write_ptr(buffer_begin, ptr_sub(out, buffer_begin)));

                        {
                            let buf = std::slice::from_raw_buf::<u8>(&run_start, count);
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

   fn flush(&mut self) -> std::io::IoResult<()> { self.inner.flush() }
}

pub fn write_packed_message<T: io::BufferedOutputStream, U: MessageBuilder>(
    output : &mut T, message : &U) -> std::io::IoResult<()> {
    let mut packed_output_stream = PackedOutputStream {inner : output};
    serialize::write_message(&mut packed_output_stream, message)
}


pub fn write_packed_message_unbuffered<T: std::io::Writer, U: MessageBuilder>(
    output : &mut T, message : &U) -> std::io::IoResult<()> {
    let mut buffered = io::BufferedOutputStreamWrapper::new(output);
    try!(write_packed_message(&mut buffered, message));
    buffered.flush()
}
