/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use io;
use message::*;
use serialize::*;

pub struct PackedInputStream<'self, T> {
    inner : &'self mut T
}

impl <'self, T : std::rt::io::Reader> std::rt::io::Reader for PackedInputStream<'self, T> {
    fn eof(&mut self) -> bool {
        self.inner.eof()
    }

    fn read(&mut self, outBuf: &mut [u8]) -> Option<uint> {
        let len = outBuf.len();

        if (len == 0) { return Some(0); }

        assert!(len % 8 == 0, "PackInputStream reads must be word-aligned");

        let mut outPos = 0;
        while (outPos < len && ! self.inner.eof() ) {

            let tag : u8 = self.inner.read_u8();

            for n in range(0, 8) {
                let isNonzero = (tag & (1 as u8 << n)) != 0;//..as bool;
                if (isNonzero) {
                    // TODO capnproto-c++ gets away without using a
                    // conditional here. Can we do something like that
                    // and would it speed things up?
                    outBuf[outPos] = self.inner.read_u8();
                    outPos += 1;
                } else {
                    outBuf[outPos] = 0;
                    outPos += 1;
                }
            }

            if (tag == 0) {

                let runLength : uint = self.inner.read_u8() as uint * 8;
                assert!(runLength <= outBuf.len() - outPos);
                unsafe {
                    std::ptr::set_memory(outBuf.unsafe_mut_ref(outPos),
                                         0, runLength);
                };
                outPos += runLength;

            } else if (tag == 0xff) {
                let runLength : uint = self.inner.read_u8() as uint * 8;

                let mut bytes_read = 0;
                while bytes_read < runLength {
                    let pos = outPos + bytes_read;
                    match self.inner.read(outBuf.mut_slice(pos, outPos + runLength)) {
                        Some(n) => bytes_read += n,
                        None => fail!("failed to read bytes")
                    }
                }
                outPos += runLength;
            }

        }

        return Some(outPos);
    }

}

pub struct PackedOutputStream<'self, W> {
    inner : &'self mut io::BufferedOutputStream<'self, W>
}

#[inline]
fn ptr_inc(p : &mut *mut u8, count : int) {
    unsafe {
        *p = std::ptr::mut_offset(*p, count);
    }
}

impl <'self, W : std::rt::io::Writer> std::rt::io::Writer for PackedOutputStream<'self, W> {
    fn write(&mut self, inBuf : &[u8]) {
//        let mut bufferLength = 0;
//        let mut out : *mut u8;

        let (mut out, mut bufferLength) = self.inner.getWriteBuffer();
        let mut slowBuffer : [u8,..20] = [0, ..20];
        let mut usingSlowBuffer = false;

        let mut inPos : uint = 0;
        let mut outPos : uint = 0;

        while (inPos < inBuf.len()) {

            if (bufferLength - outPos < 10) {
                //# Oops, we're out of space. We need at least 10
                //# bytes for the fast path, since we don't
                //# bounds-check on every byte.
                if (usingSlowBuffer) {
                    self.inner.write(slowBuffer.slice(0, outPos));
                } else {
                    self.inner.advance(outPos);
                }
                unsafe { out = slowBuffer.unsafe_mut_ref(0) }
                outPos = 0;
                bufferLength = 20;
                usingSlowBuffer = true;
            }

            let tagPos : *mut u8 = out;
            ptr_inc(&mut out, 1);
            outPos += 1;

            let bit0 = (inBuf[inPos] != 0) as u8;
            unsafe { *out = inBuf[inPos] }
            ptr_inc(&mut out, bit0 as int);
            outPos += bit0 as uint; inPos += 1;

            let bit1 = (inBuf[inPos] != 0) as u8;
            unsafe { *out = inBuf[inPos] }
            ptr_inc(&mut out, bit1 as int);
            outPos += bit1 as uint; inPos += 1;

            let bit2 = (inBuf[inPos] != 0) as u8;
            unsafe { *out = inBuf[inPos] }
            ptr_inc(&mut out, bit2 as int);
            outPos += bit2 as uint; inPos += 1;

            let bit3 = (inBuf[inPos] != 0) as u8;
            unsafe { *out = inBuf[inPos] }
            ptr_inc(&mut out, bit3 as int);
            outPos += bit3 as uint; inPos += 1;

            let bit4 = (inBuf[inPos] != 0) as u8;
            unsafe { *out = inBuf[inPos] }
            ptr_inc(&mut out, bit4 as int);
            outPos += bit4 as uint; inPos += 1;

            let bit5 = (inBuf[inPos] != 0) as u8;
            unsafe { *out = inBuf[inPos] }
            ptr_inc(&mut out, bit5 as int);
            outPos += bit5 as uint; inPos += 1;

            let bit6 = (inBuf[inPos] != 0) as u8;
            unsafe { *out = inBuf[inPos] }
            ptr_inc(&mut out, bit6 as int);
            outPos += bit6 as uint; inPos += 1;

            let bit7 = (inBuf[inPos] != 0) as u8;
            unsafe { *out = inBuf[inPos] }
            ptr_inc(&mut out, bit7 as int);
            outPos += bit7 as uint; inPos += 1;


            let tag : u8 = (bit0 << 0) | (bit1 << 1) | (bit2 << 2) | (bit3 << 3)
                         | (bit4 << 4) | (bit5 << 5) | (bit6 << 6) | (bit7 << 7);

            unsafe {*tagPos = tag }

            if (tag == 0) {
                //# An all-zero word is followed by a count of
                //# consecutive zero words (not including the first
                //# one).

                let mut count : u8 = 0;
                unsafe {
                    let mut inWord : *u64 =
                        std::cast::transmute(inBuf.unsafe_ref(inPos));
                    while (count < 255 && inPos < inBuf.len() && *inWord == 0) {
                        inPos += 8;
                        inWord = std::cast::transmute(inBuf.unsafe_ref(inPos));
                        count += 1;
                    }
                }
                unsafe {*out = count }
                ptr_inc(&mut out, 1);
                outPos += 1;

            } else if (tag == 0xff) {
                //# An all-nonzero word is followed by a count of
                //# consecutive uncompressed words, followed by the
                //# uncompressed words themselves.

                //# Count the number of consecutive words in the input
                //# which have no more than a single zero-byte. We look
                //# for at least two zeros because that's the point
                //# where our compression scheme becomes a net win.
                let mut count : u8 = 0;
                let runStart = inPos;
                while (inPos < inBuf.len() && count < 255) {
                    let mut c = 0;

                    for _ in range(0,8) {
                        c += (inBuf[inPos] == 0) as u8;
                        inPos += 1;
                    }

                    if (c >= 2) {
                        //# Un-read the word with multiple zeros, since
                        //# we'll want to compress that one.
                        inPos -= 8;
                        break;
                    }

                    count += 1;
                }
                unsafe { *out = count }
                ptr_inc(&mut out, 1);
                outPos += 1;

                if (count as uint * 8 <= bufferLength - outPos) {
                    //# There's enough space to memcpy.
                    unsafe {
                        let src : *u8 = inBuf.unsafe_ref(runStart);
                        std::ptr::copy_memory(out, src, 8 * count as uint);
                    }
                    ptr_inc(&mut out, count as int * 8);
                    outPos += count as uint * 8;
                } else {
                    //# Input overruns the output buffer. We'll give it
                    //# to the output stream in one chunk and let it
                    //# decide what to do.
                    if usingSlowBuffer {
                        self.inner.write(slowBuffer.slice(0, outPos));
                    } else {
                        self.inner.advance(outPos);
                    }

                    self.inner.write(inBuf.slice(runStart, runStart + 8 * count as uint));

                    let (out1, bufferLength1) = self.inner.getWriteBuffer();
                    out = out1; bufferLength = bufferLength1;
                    usingSlowBuffer = false;
                    outPos = 0;
                }
            }
        }

        if usingSlowBuffer {
            self.inner.write(slowBuffer.slice(0, outPos));
        } else {
            self.inner.advance(outPos);
        }
    }

   fn flush(&mut self) { self.inner.flush(); }
}

pub fn writePackedMessage<T : std::rt::io::Writer>(outputStream : &mut io::BufferedOutputStream<T>,
                                                   message : &MessageBuilder) {

    let mut packedOutputStream = PackedOutputStream {inner : outputStream};

    writeMessage(&mut packedOutputStream, message);
}
