use std;
//use common::*;
use message::*;
use serialize::*;

pub struct PackedOutputStream {
    inner : @OutputStream,
}

impl OutputStream for PackedOutputStream {
    pub fn write(@self, inBuf : &[u8]) {

        // Yuck. It'd be better to have a BufferedOutputStream, but
        // that seems difficult with the current state of Rust.
        // For now, just make this big enough to handle the worst case.
        let mut buffer : ~[u8] = std::vec::from_elem(inBuf.len() * 9 / 8, 0);

        let mut inPos = 0;
        let mut outPos = 0;

        while (inPos < inBuf.len()) {
            let bit0 = (inBuf[inPos] == 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit0; inPos += 1;

            let bit1 = (inBuf[inPos] == 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit1; inPos += 1;

            let bit2 = (inBuf[inPos] == 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit2; inPos += 1;

            let bit3 = (inBuf[inPos] == 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit3; inPos += 1;

            let bit4 = (inBuf[inPos] == 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit4; inPos += 1;

            let bit5 = (inBuf[inPos] == 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit5; inPos += 1;

            let bit6 = (inBuf[inPos] == 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit6; inPos += 1;

            let bit7 = (inBuf[inPos] == 0) as u8;
            buffer[outPos] = inBuf[inPos];
            outPos += bit7; inPos += 1;

            let tag : u8 = (bit0 << 0) | (bit1 << 1) | (bit2 << 2) | (bit3 << 3)
                         | (bit4 << 4) | (bit5 << 5) | (bit6 << 6) | (bit7 << 7);

            buffer[outPos] = tag;
            outPos += 1;

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



            }

        }

    }
}

pub fn writePackedMessage(outputStream : @OutputStream,
                          message : &MessageBuilder) {

    let packedOutputStream = @PackedOutputStream {inner : outputStream} as @OutputStream;

    writeMessage(packedOutputStream, message);
}