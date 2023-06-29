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

//! Reading and writing of messages using the
//! [packed stream encoding](https://capnproto.org/encoding.html#packing).

use crate::io::{BufRead, Read, Write};
use core::{mem, ptr, slice};

#[cfg(feature = "alloc")]
use crate::message;
#[cfg(feature = "alloc")]
use crate::serialize;
use crate::{Error, ErrorKind, Result};

/// A `BufRead` wrapper that unpacks packed data. Returns an error on any `read()`
/// call that would end within an all-zero (tag 0x00) or uncompressed (tag 0xff)
/// run of words. Calls that come from `serialize_packed::read_message()` and
/// `serialize_packed::try_read_message()` always mirror `write()` calls from
/// `serialize_packed::write_message()`, so they always safely span such runs.
struct PackedRead<R>
where
    R: BufRead,
{
    inner: R,
}

impl<R> PackedRead<R>
where
    R: BufRead,
{
    fn get_read_buffer(&mut self) -> Result<(*const u8, *const u8)> {
        let buf = self.inner.fill_buf()?;
        Ok((buf.as_ptr(), buf.as_ptr().wrapping_add(buf.len())))
    }
}

#[inline]
fn ptr_sub<T>(p1: *const T, p2: *const T) -> usize {
    (p1 as usize - p2 as usize) / mem::size_of::<T>()
}

macro_rules! refresh_buffer(
    ($this:expr, $size:ident, $in_ptr:ident, $in_end:ident, $out:ident,
     $outBuf:ident, $buffer_begin:ident) => (
        {
            $this.inner.consume($size);
            let (b, e) = $this.get_read_buffer()?;
            $in_ptr = b;
            $in_end = e;
            $size = ptr_sub($in_end, $in_ptr);
            $buffer_begin = b;
            if $size == 0 {
                return Err(Error::from_kind(ErrorKind::PrematureEndOfPackedInput));
            }
        }
        );
    );

impl<R> Read for PackedRead<R>
where
    R: BufRead,
{
    fn read(&mut self, out_buf: &mut [u8]) -> Result<usize> {
        let len = out_buf.len();
        if len == 0 {
            return Ok(0);
        }

        assert!(len % 8 == 0, "PackedRead reads must be word-aligned.");

        unsafe {
            let out_buf_start = out_buf.as_mut_ptr();
            let mut out = out_buf_start;
            let out_end: *mut u8 = out.wrapping_add(len);

            let (mut in_ptr, mut in_end) = self.get_read_buffer()?;
            let mut buffer_begin = in_ptr;
            let mut size = ptr_sub(in_end, in_ptr);
            if size == 0 {
                return Ok(0);
            }

            loop {
                let tag: u8;

                assert_eq!(
                    ptr_sub(out, out_buf_start) % 8,
                    0,
                    "Output pointer should always be aligned here."
                );

                if ptr_sub(in_end, in_ptr) < 10 {
                    if ptr_sub(in_end, in_ptr) == 0 {
                        refresh_buffer!(self, size, in_ptr, in_end, out, out_buf, buffer_begin);
                        continue;
                    }

                    //# We have at least 1, but not 10, bytes available. We need to read
                    //# slowly, doing a bounds check on each byte.

                    tag = *in_ptr;
                    in_ptr = in_ptr.offset(1);

                    for i in 0..8 {
                        if (tag & (1u8 << i)) != 0 {
                            if ptr_sub(in_end, in_ptr) == 0 {
                                refresh_buffer!(
                                    self,
                                    size,
                                    in_ptr,
                                    in_end,
                                    out,
                                    out_buf,
                                    buffer_begin
                                );
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
                        refresh_buffer!(self, size, in_ptr, in_end, out, out_buf, buffer_begin);
                    }
                } else {
                    tag = *in_ptr;
                    in_ptr = in_ptr.offset(1);

                    for n in 0..8 {
                        let is_nonzero = (tag & (1u8 << n)) != 0;
                        *out = (*in_ptr) & ((-i8::from(is_nonzero)) as u8);
                        out = out.offset(1);
                        in_ptr = in_ptr.offset(isize::from(is_nonzero));
                    }
                }
                if tag == 0 {
                    assert!(
                        ptr_sub(in_end, in_ptr) > 0,
                        "Should always have non-empty buffer here."
                    );

                    let run_length: usize = (*in_ptr) as usize * 8;
                    in_ptr = in_ptr.offset(1);

                    if run_length > ptr_sub(out_end, out) {
                        return Err(Error::from_kind(
                            ErrorKind::PackedInputDidNotEndCleanlyOnASegmentBoundary,
                        ));
                    }

                    ptr::write_bytes(out, 0, run_length);
                    out = out.add(run_length);
                } else if tag == 0xff {
                    assert!(
                        ptr_sub(in_end, in_ptr) > 0,
                        "Should always have non-empty buffer here"
                    );

                    let mut run_length: usize = (*in_ptr) as usize * 8;
                    in_ptr = in_ptr.offset(1);

                    if run_length > ptr_sub(out_end, out) {
                        return Err(Error::from_kind(
                            ErrorKind::PackedInputDidNotEndCleanlyOnASegmentBoundary,
                        ));
                    }

                    let in_remaining = ptr_sub(in_end, in_ptr);
                    if in_remaining >= run_length {
                        //# Fast path.
                        ptr::copy_nonoverlapping(in_ptr, out, run_length);
                        out = out.add(run_length);
                        in_ptr = in_ptr.add(run_length);
                    } else {
                        //# Copy over the first buffer, then do one big read for the rest.
                        ptr::copy_nonoverlapping(in_ptr, out, in_remaining);
                        out = out.add(in_remaining);
                        run_length -= in_remaining;

                        self.inner.consume(size);
                        {
                            let buf = slice::from_raw_parts_mut::<u8>(out, run_length);
                            self.inner.read_exact(buf)?;
                        }

                        out = out.add(run_length);

                        if out == out_end {
                            return Ok(len);
                        } else {
                            let (b, e) = self.get_read_buffer()?;
                            in_ptr = b;
                            in_end = e;
                            size = ptr_sub(e, b);
                            buffer_begin = in_ptr;
                            continue;
                        }
                    }
                }

                if out == out_end {
                    self.inner.consume(ptr_sub(in_ptr, buffer_begin));
                    return Ok(len);
                }
            }
        }
    }
}

/// Reads a packed message from a stream using the provided options.
#[cfg(feature = "alloc")]
pub fn read_message<R>(
    read: R,
    options: message::ReaderOptions,
) -> Result<crate::message::Reader<serialize::OwnedSegments>>
where
    R: BufRead,
{
    let packed_read = PackedRead { inner: read };
    serialize::read_message(packed_read, options)
}

/// Like read_message(), but returns None instead of an error if there are zero bytes left in `read`.
#[cfg(feature = "alloc")]
pub fn try_read_message<R>(
    read: R,
    options: message::ReaderOptions,
) -> Result<Option<crate::message::Reader<serialize::OwnedSegments>>>
where
    R: BufRead,
{
    let packed_read = PackedRead { inner: read };
    serialize::try_read_message(packed_read, options)
}

struct PackedWrite<W>
where
    W: Write,
{
    inner: W,
}

impl<W> Write for PackedWrite<W>
where
    W: Write,
{
    fn write_all(&mut self, in_buf: &[u8]) -> Result<()> {
        unsafe {
            let mut buf_idx: usize = 0;
            let mut buf: [u8; 64] = [0; 64];

            let mut in_ptr: *const u8 = in_buf.as_ptr();
            let in_end: *const u8 = in_buf.as_ptr().wrapping_add(in_buf.len());

            while in_ptr < in_end {
                if buf_idx + 10 > buf.len() {
                    //# Oops, we're out of space. We need at least 10
                    //# bytes for the fast path, since we don't
                    //# bounds-check on every byte.
                    self.inner.write_all(&buf[..buf_idx])?;
                    buf_idx = 0;
                }

                let tag_pos = buf_idx;
                buf_idx += 1;

                let bit0 = u8::from(*in_ptr != 0);
                *buf.get_unchecked_mut(buf_idx) = *in_ptr;
                buf_idx += bit0 as usize;
                in_ptr = in_ptr.offset(1);

                let bit1 = u8::from(*in_ptr != 0);
                *buf.get_unchecked_mut(buf_idx) = *in_ptr;
                buf_idx += bit1 as usize;
                in_ptr = in_ptr.offset(1);

                let bit2 = u8::from(*in_ptr != 0);
                *buf.get_unchecked_mut(buf_idx) = *in_ptr;
                buf_idx += bit2 as usize;
                in_ptr = in_ptr.offset(1);

                let bit3 = u8::from(*in_ptr != 0);
                *buf.get_unchecked_mut(buf_idx) = *in_ptr;
                buf_idx += bit3 as usize;
                in_ptr = in_ptr.offset(1);

                let bit4 = u8::from(*in_ptr != 0);
                *buf.get_unchecked_mut(buf_idx) = *in_ptr;
                buf_idx += bit4 as usize;
                in_ptr = in_ptr.offset(1);

                let bit5 = u8::from(*in_ptr != 0);
                *buf.get_unchecked_mut(buf_idx) = *in_ptr;
                buf_idx += bit5 as usize;
                in_ptr = in_ptr.offset(1);

                let bit6 = u8::from(*in_ptr != 0);
                *buf.get_unchecked_mut(buf_idx) = *in_ptr;
                buf_idx += bit6 as usize;
                in_ptr = in_ptr.offset(1);

                let bit7 = u8::from(*in_ptr != 0);
                *buf.get_unchecked_mut(buf_idx) = *in_ptr;
                buf_idx += bit7 as usize;
                in_ptr = in_ptr.offset(1);

                let tag: u8 = bit0
                    | (bit1 << 1)
                    | (bit2 << 2)
                    | (bit3 << 3)
                    | (bit4 << 4)
                    | (bit5 << 5)
                    | (bit6 << 6)
                    | (bit7 << 7);

                *buf.get_unchecked_mut(tag_pos) = tag;

                if tag == 0 {
                    //# An all-zero word is followed by a count of
                    //# consecutive zero words (not including the first
                    //# one).

                    let mut in_word: *const [u8; 8] = in_ptr as *const [u8; 8];
                    let mut limit: *const [u8; 8] = in_end as *const [u8; 8];
                    if ptr_sub(limit, in_word) > 255 {
                        limit = in_word.offset(255);
                    }
                    while in_word < limit && *in_word == [0; 8] {
                        in_word = in_word.offset(1);
                    }

                    *buf.get_unchecked_mut(buf_idx) =
                        ptr_sub(in_word, in_ptr as *const [u8; 8]) as u8;
                    buf_idx += 1;
                    in_ptr = in_word as *const u8;
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
                            c += u8::from(*in_ptr == 0);
                            in_ptr = in_ptr.offset(1);
                        }

                        if c >= 2 {
                            //# Un-read the word with multiple zeros, since
                            //# we'll want to compress that one.
                            in_ptr = in_ptr.offset(-8);
                            break;
                        }
                    }

                    let count: usize = ptr_sub(in_ptr, run_start);
                    *buf.get_unchecked_mut(buf_idx) = (count / 8) as u8;
                    buf_idx += 1;

                    self.inner.write_all(&buf[..buf_idx])?;
                    buf_idx = 0;
                    self.inner
                        .write_all(slice::from_raw_parts::<u8>(run_start, count))?;
                }
            }

            self.inner.write_all(&buf[..buf_idx])?;
            Ok(())
        }
    }
}

/// Writes a packed message to a stream.
///
/// The only source of errors from this function are `write.write_all()` calls. If you pass in
/// a writer that never returns an error, then this function will never return an error.
#[cfg(feature = "alloc")]
pub fn write_message<W, A>(write: W, message: &crate::message::Builder<A>) -> Result<()>
where
    W: Write,
    A: crate::message::Allocator,
{
    let packed_write = PackedWrite { inner: write };
    serialize::write_message(packed_write, message)
}

#[cfg(feature = "alloc")]
#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use crate::io::{Read, Write};

    use quickcheck::{quickcheck, TestResult};

    use super::read_message;
    use crate::message::ReaderOptions;
    use crate::serialize::test::write_message_segments;
    use crate::serialize_packed::{PackedRead, PackedWrite};
    use crate::ErrorKind;

    #[test]
    pub fn premature_eof() {
        let input_bytes: &[u8] = &[];
        let mut packed_read = PackedRead { inner: input_bytes };

        let mut output_bytes: Vec<u8> = vec![0; 8];
        assert!(packed_read.read_exact(&mut output_bytes[..]).is_err());
    }

    pub fn check_unpacks_to(packed: &[u8], unpacked: &[u8]) {
        let mut packed_read = PackedRead { inner: packed };

        let mut bytes: Vec<u8> = vec![0; unpacked.len()];
        packed_read.read_exact(&mut bytes[..]).unwrap();

        assert!(packed_read.inner.is_empty()); // nothing left to read
        assert_eq!(bytes, unpacked);
    }

    pub fn check_packing(unpacked: &[u8], packed: &[u8]) {
        // --------
        // write

        let mut bytes: Vec<u8> = vec![0; packed.len()];
        {
            let mut packed_write = PackedWrite {
                inner: &mut bytes[..],
            };
            packed_write.write_all(unpacked).unwrap();
        }

        assert_eq!(bytes, packed);

        // --------
        // read
        check_unpacks_to(packed, unpacked);
    }

    #[test]
    pub fn simple_packing() {
        check_packing(&[], &[]);
        check_packing(&[0; 8], &[0, 0]);
        check_packing(&[0, 0, 12, 0, 0, 34, 0, 0], &[0x24, 12, 34]);
        check_packing(
            &[1, 3, 2, 4, 5, 7, 6, 8],
            &[0xff, 1, 3, 2, 4, 5, 7, 6, 8, 0],
        );
        check_packing(
            &[0, 0, 0, 0, 0, 0, 0, 0, 1, 3, 2, 4, 5, 7, 6, 8],
            &[0, 0, 0xff, 1, 3, 2, 4, 5, 7, 6, 8, 0],
        );
        check_packing(
            &[0, 0, 12, 0, 0, 34, 0, 0, 1, 3, 2, 4, 5, 7, 6, 8],
            &[0x24, 12, 34, 0xff, 1, 3, 2, 4, 5, 7, 6, 8, 0],
        );
        check_packing(
            &[1, 3, 2, 4, 5, 7, 6, 8, 8, 6, 7, 4, 5, 2, 3, 1],
            &[0xff, 1, 3, 2, 4, 5, 7, 6, 8, 1, 8, 6, 7, 4, 5, 2, 3, 1],
        );

        check_packing(
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4,
                5, 6, 7, 8, 0, 2, 4, 0, 9, 0, 5, 1,
            ],
            &[
                0xff, 1, 2, 3, 4, 5, 6, 7, 8, 3, 1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8, 1,
                2, 3, 4, 5, 6, 7, 8, 0xd6, 2, 4, 9, 5, 1,
            ],
        );
        check_packing(
            &[
                1, 2, 3, 4, 5, 6, 7, 8, 1, 2, 3, 4, 5, 6, 7, 8, 6, 2, 4, 3, 9, 0, 5, 1, 1, 2, 3, 4,
                5, 6, 7, 8, 0, 2, 4, 0, 9, 0, 5, 1,
            ],
            &[
                0xff, 1, 2, 3, 4, 5, 6, 7, 8, 3, 1, 2, 3, 4, 5, 6, 7, 8, 6, 2, 4, 3, 9, 0, 5, 1, 1,
                2, 3, 4, 5, 6, 7, 8, 0xd6, 2, 4, 9, 5, 1,
            ],
        );

        check_packing(
            &[
                8, 0, 100, 6, 0, 1, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 0, 3, 1,
            ],
            &[0xed, 8, 100, 6, 1, 1, 2, 0, 2, 0xd4, 1, 2, 3, 1],
        );

        check_packing(&[0; 16], &[0, 1]);
        check_packing(
            &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            &[0, 2],
        );
    }

    quickcheck! {
        #[cfg_attr(miri, ignore)] // miri takes a long time with quickcheck
        fn test_round_trip(segments: Vec<Vec<crate::Word>>) -> TestResult {
            use crate::message::ReaderSegments;
            if segments.is_empty() { return TestResult::discard(); }
            let mut buf: Vec<u8> = Vec::new();

            write_message_segments(&mut PackedWrite { inner: &mut buf }, &segments);
            let message = read_message(&mut &buf[..], ReaderOptions::new()).unwrap();
            let result_segments = message.into_segments();

            TestResult::from_bool(segments.iter().enumerate().all(|(i, segment)| {
                crate::Word::words_to_bytes(&segment[..]) == result_segments.get_segment(i as u32).unwrap()
            }))
        }

        #[cfg_attr(miri, ignore)] // miri takes a long time with quickcheck
        fn test_unpack(packed: Vec<u8>) -> TestResult {
            let len = packed.len();
            let mut packed_read = PackedRead { inner: &packed[..] };

            let mut out_buffer: Vec<u8> = vec![0; len * 8];

            let _ = packed_read.read_exact(&mut out_buffer);
            TestResult::from_bool(true)
        }
    }

    #[test]
    fn did_not_end_cleanly_on_a_segment_boundary() {
        let packed = &[0xff, 1, 2, 3, 4, 5, 6, 7, 8, 37, 1, 2];
        let mut packed_read = PackedRead { inner: &packed[..] };

        let mut bytes: Vec<u8> = vec![0; 200];
        match packed_read.read_exact(&mut bytes[..]) {
            Ok(_) => panic!("should have been an error"),
            Err(e) => {
                assert_eq!(
                    e.kind,
                    ErrorKind::PackedInputDidNotEndCleanlyOnASegmentBoundary,
                );
            }
        }
    }

    #[test]
    fn premature_end_of_packed_input() {
        fn helper(packed: &[u8]) {
            let mut packed_read = PackedRead { inner: packed };

            let mut bytes: Vec<u8> = vec![0; 200];
            match packed_read.read_exact(&mut bytes[..]) {
                Ok(_) => panic!("should have been an error"),
                Err(e) => {
                    assert_eq!(e.kind, ErrorKind::PrematureEndOfPackedInput);
                }
            }
        }

        helper(&[0xf0, 1, 2]);
        helper(&[0]);
        helper(&[0xff, 1, 2, 3, 4, 5, 6, 7, 8]);

        // In this case, the error is only due to the fact that the unpacked data does not
        // fill up the given output buffer.
        helper(&[1, 1]);
    }

    #[test]
    fn packed_segment_table() {
        let packed_buf = &[0x11, 4, 1, 0, 1, 0, 0];

        check_unpacks_to(
            packed_buf,
            &[
                4, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0,
            ],
        );

        // At one point, this failed due to serialize::read_message()
        // reading the segment table only one word at a time.
        read_message(&mut &packed_buf[..], Default::default()).unwrap();
    }
}
