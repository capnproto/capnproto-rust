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

impl <'self, T : std::io::Reader> std::io::Reader for PackedInputStream<'self, T> {
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

pub struct PackedOutputStream<'a, 'b, W> {
    inner : &'a mut io::BufferedOutputStream<'b, W>
}

#[inline]
fn ptr_inc(p : &mut *mut u8, count : int) {
    unsafe {
        *p = std::ptr::mut_offset(*p, count);
    }
}

#[inline]
fn ptr_sub<T>(p1 : * T, p2 : * T) -> uint {
    unsafe {
        let p1Addr : uint = std::cast::transmute(p1);
        let p2Addr : uint = std::cast::transmute(p2);
        return (p1Addr - p2Addr) / std::mem::size_of::<T>();
    }
}

#[inline]
fn mut_ptr_sub <T>(p1 : *mut T, p2 : *mut T) -> uint {
    unsafe {
        let p1Addr : uint = std::cast::transmute(p1);
        let p2Addr : uint = std::cast::transmute(p2);
        return (p1Addr - p2Addr) / std::mem::size_of::<T>();
    }
}

impl <'a, 'b, W : std::io::Writer> std::io::Writer for PackedOutputStream<'a, 'b, W> {
    fn write(&mut self, inBuf : &[u8]) {
        unsafe {
            let (mut out, mut bufferEnd) = self.inner.getWriteBuffer();
            let mut bufferBegin = out;
            let mut slowBuffer : [u8,..20] = [0, ..20];

            let mut inPtr : *u8 = inBuf.unsafe_ref(0);
            let inEnd : *u8 = inBuf.unsafe_ref(inBuf.len());

            while (inPtr < inEnd) {

                if (mut_ptr_sub(bufferEnd, out) < 10) {
                    //# Oops, we're out of space. We need at least 10
                    //# bytes for the fast path, since we don't
                    //# bounds-check on every byte.
                    self.inner.write_ptr(bufferBegin, mut_ptr_sub(out, bufferBegin));

                    out = slowBuffer.unsafe_mut_ref(0);
                    bufferEnd = slowBuffer.unsafe_mut_ref(20);
                    bufferBegin = out;
                }

                let tagPos : *mut u8 = out;
                ptr_inc(&mut out, 1);

                let bit0 = (*inPtr != 0) as u8;
                *out = *inPtr;
                ptr_inc(&mut out, bit0 as int);
                inPtr = std::ptr::offset(inPtr, 1);

                let bit1 = (*inPtr != 0) as u8;
                *out = *inPtr;
                ptr_inc(&mut out, bit1 as int);
                inPtr = std::ptr::offset(inPtr, 1);

                let bit2 = (*inPtr != 0) as u8;
                *out = *inPtr;
                ptr_inc(&mut out, bit2 as int);
                inPtr = std::ptr::offset(inPtr, 1);

                let bit3 = (*inPtr != 0) as u8;
                *out = *inPtr;
                ptr_inc(&mut out, bit3 as int);
                inPtr = std::ptr::offset(inPtr, 1);

                let bit4 = (*inPtr != 0) as u8;
                *out = *inPtr;
                ptr_inc(&mut out, bit4 as int);
                inPtr = std::ptr::offset(inPtr, 1);

                let bit5 = (*inPtr != 0) as u8;
                *out = *inPtr;
                ptr_inc(&mut out, bit5 as int);
                inPtr = std::ptr::offset(inPtr, 1);

                let bit6 = (*inPtr != 0) as u8;
                *out = *inPtr;
                ptr_inc(&mut out, bit6 as int);
                inPtr = std::ptr::offset(inPtr, 1);

                let bit7 = (*inPtr != 0) as u8;
                *out = *inPtr;
                ptr_inc(&mut out, bit7 as int);
                inPtr = std::ptr::offset(inPtr, 1);


                let tag : u8 = (bit0 << 0) | (bit1 << 1) | (bit2 << 2) | (bit3 << 3)
                    | (bit4 << 4) | (bit5 << 5) | (bit6 << 6) | (bit7 << 7);

                *tagPos = tag;

                if (tag == 0) {
                    //# An all-zero word is followed by a count of
                    //# consecutive zero words (not including the first
                    //# one).

                    let mut inWord : *u64 = std::cast::transmute(inPtr);
                    let mut limit : *u64 = std::cast::transmute(inEnd);
                    if (ptr_sub(limit, inWord) > 255) {
                        limit = std::ptr::offset(inWord, 255);
                    }
                    while (inWord < limit && *inWord == 0) {
                        inWord = std::ptr::offset(inWord, 1)
                    }
                    *out = ptr_sub(inWord, std::cast::transmute::<*u8, *u64>(inPtr)) as u8;

                    ptr_inc(&mut out, 1);
                    inPtr = std::cast::transmute::<*u64, *u8>(inWord);

                } else if (tag == 0xff) {
                    //# An all-nonzero word is followed by a count of
                    //# consecutive uncompressed words, followed by the
                    //# uncompressed words themselves.

                    //# Count the number of consecutive words in the input
                    //# which have no more than a single zero-byte. We look
                    //# for at least two zeros because that's the point
                    //# where our compression scheme becomes a net win.
                    let runStart = inPtr;
                    let mut limit = inEnd;
                    if (ptr_sub(limit, inPtr) > 255 * 8) {
                        limit = std::ptr::offset(inPtr, 255 * 8);
                    }

                    while (inPtr < limit) {
                        let mut c = 0;

                        for _ in range(0,8) {
                            c += (*inPtr == 0) as u8;
                            inPtr = std::ptr::offset(inPtr, 1);
                        }

                        if (c >= 2) {
                            //# Un-read the word with multiple zeros, since
                            //# we'll want to compress that one.
                            inPtr = std::ptr::offset(inPtr, -8);
                            break;
                        }
                    }
                    let count : uint = ptr_sub(inPtr, runStart);
                    *out = (count / 8) as u8;
                    ptr_inc(&mut out, 1);

                    if (count <= mut_ptr_sub(bufferEnd, out)) {
                        //# There's enough space to memcpy.

                        let src : *u8 = runStart;
                        std::ptr::copy_memory(out, src, count);

                        ptr_inc(&mut out, count as int);
                    } else {
                        //# Input overruns the output buffer. We'll give it
                        //# to the output stream in one chunk and let it
                        //# decide what to do.
                        self.inner.write_ptr(bufferBegin, mut_ptr_sub(out, bufferBegin));

                        do std::vec::raw::buf_as_slice::<u8,()>(runStart, count) |buf| {
                            self.inner.write(buf);
                        }

                        let (out1, bufferEnd1) = self.inner.getWriteBuffer();
                        out = out1; bufferEnd = bufferEnd1;
                        bufferBegin = out;
                    }
                }
            }

            self.inner.write_ptr(bufferBegin, mut_ptr_sub(out, bufferBegin));
            self.inner.flush();
        }
    }

   fn flush(&mut self) { self.inner.flush(); }
}

pub trait WritePacked {
    fn writePackedMessage(&mut self, message : &MessageBuilder);
}

impl <'self, T : std::io::Writer> WritePacked for io::BufferedOutputStream<'self, T> {
    fn writePackedMessage(&mut self, message : &MessageBuilder) {
        let mut packedOutputStream = PackedOutputStream {inner : self};
        writeMessage(&mut packedOutputStream, message);
    }
}

pub struct WritePackedWrapper<'a, T> {writer : &'a mut T }

impl <'a, T: std::io::Writer> WritePacked for WritePackedWrapper<'a, T> {
    fn writePackedMessage(&mut self, message : &MessageBuilder) {
        let mut buffered = io::BufferedOutputStream::new(self.writer);
        buffered.writePackedMessage(message);
    }
}
