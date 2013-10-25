/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
//use common::*;
use message::*;
use serialize::*;
use std::rt::io::ReaderByteConversions;

pub struct PackedInputStream<T> {
    inner : T
}

impl <T : std::rt::io::Reader> std::rt::io::Reader for PackedInputStream<T> {
    fn eof(&mut self) -> bool {
        self.inner.eof()
    }

    fn read(&mut self, outBuf: &mut [u8]) -> Option<uint> {
        let len = outBuf.len();

        if (len == 0) { return Some(0); }

        assert!(len % 8 == 0, "PackInputStream reads must be word-aligned");

        let mut outPos = 0;
        while (outPos < len && ! self.inner.eof() ) {

            let tag : u8 = self.inner.read_u8_();

            for n in range(0, 8) {
                let isNonzero = (tag & (1 as u8 << n)) != 0;//..as bool;
                if (isNonzero) {
                    // TODO capnproto-c++ gets away without using a
                    // conditional here. Can we do something like that
                    // and would it speed things up?
                    outBuf[outPos] = self.inner.read_u8_();
                    outPos += 1;
                } else {
                    outBuf[outPos] = 0;
                    outPos += 1;
                }
            }

            if (tag == 0) {

                let runLength : uint = self.inner.read_u8_() as uint * 8;

                unsafe {
                    std::ptr::set_memory(outBuf.unsafe_mut_ref(outPos),
                                         0, runLength);
                };
                outPos += runLength;

            } else if (tag == 0xff) {
                let runLength : uint = self.inner.read_u8_() as uint * 8;

                let mut bytes_read = 0;
                loop {
                    if bytes_read >= runLength { break }
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

pub struct PackedOutputStream<T> {
    inner : T
}

impl <T : std::rt::io::Writer> std::rt::io::Writer for PackedOutputStream<T> {
    fn write(&mut self, inBuf : &[u8]) {

        // Yuck. It'd be better to have a BufferedOutputStream, but
        // that seems difficult with the current state of Rust.
        // For now, just make this big enough to handle the worst case.
        let mut buffer : ~[u8] = std::vec::from_elem(inBuf.len() * 3 / 2, 0u8);

        let mut inPos : uint = 0;
        let mut outPos : uint = 0;

        while (inPos < inBuf.len()) {

            let tagPos = outPos;
            outPos += 1;

            let bit0 = (inBuf[inPos] != 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit0 as uint; inPos += 1;

            let bit1 = (inBuf[inPos] != 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit1 as uint; inPos += 1;

            let bit2 = (inBuf[inPos] != 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit2 as uint; inPos += 1;

            let bit3 = (inBuf[inPos] != 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit3 as uint; inPos += 1;

            let bit4 = (inBuf[inPos] != 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit4 as uint; inPos += 1;

            let bit5 = (inBuf[inPos] != 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit5 as uint; inPos += 1;

            let bit6 = (inBuf[inPos] != 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit6 as uint; inPos += 1;

            let bit7 = (inBuf[inPos] != 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit7 as uint; inPos += 1;


            let tag : u8 = (bit0 << 0) | (bit1 << 1) | (bit2 << 2) | (bit3 << 3)
                         | (bit4 << 4) | (bit5 << 5) | (bit6 << 6) | (bit7 << 7);

            buffer[tagPos] = tag;

            if (tag == 0) {
                //# An all-zero word is followed by a count of
                //# consecutive zero words (not including the first
                //# one).

                let mut count : u8 = 0;
                unsafe {
                    let mut inWord : *u64 =
                        std::cast::transmute(inBuf.unsafe_ref(inPos));
                    while (count < 255 && *inWord == 0) {
                        inWord = std::ptr::offset(inWord, 1);
                        count += 1;
                    }
                }
                buffer[outPos] = count;
                outPos += 1;

                inPos += count as uint * 8;

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
                buffer[outPos] = count;
                outPos += 1;

                unsafe {
                    let dst : *mut u8 = buffer.unsafe_mut_ref(outPos);
                    let src : *u8 = inBuf.unsafe_ref(runStart);
                    std::ptr::copy_memory(dst, src, 8 * count as uint);
                }
                outPos += count as uint * 8;

            }
        }

        self.inner.write(buffer.slice(0, outPos));
    }

   fn flush(&mut self) { self.inner.flush(); }
}

pub fn writePackedMessage<T : std::rt::io::Writer>(outputStream : T,
                                                   message : &MessageBuilder) {

    let mut packedOutputStream = PackedOutputStream {inner : outputStream};

    writeMessage(&mut packedOutputStream, message);
}
