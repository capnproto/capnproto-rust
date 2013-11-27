/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use common::*;
use endian::*;
use message::*;

pub mod InputStreamMessageReader {

    use std;
    use io;
    use arena::SegmentReader;
    use common::*;
    use endian::*;
    use message::*;

    pub fn new<U : std::io::Reader, T>(inputStream : &mut U,
                                           options : ReaderOptions,
                                           cont : |&mut MessageReader| -> T) -> T {

        let firstWord = inputStream.read_bytes(8);

        let segmentCount : u32 =
            unsafe {let p : *WireValue<u32> = std::cast::transmute(firstWord.unsafe_ref(0));
                    (*p).get() + 1
                   };


        let segment0Size =
            if (segmentCount == 0) { 0 } else {
            unsafe {let p : *WireValue<u32> = std::cast::transmute(firstWord.unsafe_ref(4));
                    (*p).get()
                   }
            };

        let mut totalWords = segment0Size;

        if (segmentCount >= 512) {
            fail!("too many segments");
        }

        let mut moreSizes : ~[u32] = std::vec::from_elem((segmentCount & !1) as uint, 0u32);

        if (segmentCount > 1) {
            let moreSizesRaw = inputStream.read_bytes((4 * (segmentCount & !1)) as uint);
            for ii in range(0, segmentCount as uint - 1) {
                moreSizes[ii] = unsafe {
                    let p : *WireValue<u32> =
                        std::cast::transmute(moreSizesRaw.unsafe_ref(ii * 4));
                    (*p).get()
                };
                totalWords += moreSizes[ii];
            }
        }

        //# Don't accept a message which the receiver couldn't possibly
        //# traverse without hitting the traversal limit. Without this
        //# check, a malicious client could transmit a very large
        //# segment size to make the receiver allocate excessive space
        //# and possibly crash.
        assert!(totalWords as u64 <= options.traversalLimitInWords);

        let mut ownedSpace : ~[Word] = allocate_zeroed_words(totalWords as uint);
        let bufLen = totalWords as uint * BYTES_PER_WORD;

        unsafe {
            let ptr : *mut u8 = std::cast::transmute(ownedSpace.unsafe_mut_ref(0));
            std::vec::raw::mut_buf_as_slice::<u8,()>(ptr, bufLen, |buf| {
                io::read_at_least(inputStream, buf, bufLen);
            })
        }

        // TODO lazy reading like in capnp-c++. Is that possible
        // within the std::io::Reader interface?

        let segment0 : &[Word] = ownedSpace.slice(0, segment0Size as uint);

        let mut segments : ~[&[Word]] = ~[segment0];

        if (segmentCount > 1) {
            let mut offset = segment0Size;

            for ii in range(0, segmentCount as uint - 1) {
                segments.push(ownedSpace.slice(offset as uint,
                                               (offset + moreSizes[ii]) as uint));
                offset += moreSizes[ii];
            }
        }

        let mut result = ~MessageReader {
            segments : segments,
            segmentReader0:
                SegmentReader {
                    messageReader : std::ptr::null(),
                    segment: segment0
                },
            moreSegmentReaders : None,
            options : options
        };


        result.segmentReader0.messageReader = std::ptr::to_unsafe_ptr(result);

        if (segmentCount > 1 ) {
            let mut moreSegmentReaders = ~[];
            for segment in segments.slice_from(1).iter() {
                let segmentReader =
                    SegmentReader {
                    messageReader : std::ptr::to_unsafe_ptr(result),
                    segment: *segment
                };
                moreSegmentReaders.push(segmentReader);
            }
            result.moreSegmentReaders = Some(moreSegmentReaders);
        }

        cont(result)
    }
}

pub fn write_message<T: std::io::Writer>(outputStream : &mut T,
                                        message : &MessageBuilder) {

    let tableSize : uint = ((message.segments.len() + 2) & (!1));

    let mut table : ~[WireValue<u32>] = std::vec::with_capacity(tableSize);
    unsafe { std::vec::raw::set_len(&mut table, tableSize) }

    table[0].set((message.segments.len() - 1) as u32);

    for i in range(0, message.segments.len()) {
        table[i + 1].set(message.segment_builders[i].pos as u32);
    }
    if (message.segments.len() % 2 == 0) {
        // Set padding.
        table[message.segments.len() + 1].set( 0 );
    }

    unsafe {
        let ptr : *u8 = std::cast::transmute(table.unsafe_ref(0));
        std::vec::raw::buf_as_slice::<u8,()>(ptr, table.len() * 4, |buf| {
            outputStream.write(buf);
        })
    }

    for i in range(0, message.segments.len()) {
        unsafe {
            let ptr : *u8 = std::cast::transmute(message.segments[i].unsafe_ref(0));
            std::vec::raw::buf_as_slice::<u8,()>(
                ptr,
                message.segment_builders[i].pos * BYTES_PER_WORD,
                |buf| { outputStream.write(buf) });
        }
    }
}
