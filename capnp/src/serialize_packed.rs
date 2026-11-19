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

use crate::message;
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
    // Called only after consuming every byte of the current input buffer.
    fn refill(&mut self, buffer_len: &mut usize) -> Result<&[u8]> {
        self.inner.consume(*buffer_len);
        let input = self.inner.fill_buf()?;
        if input.is_empty() {
            return Err(Error::from_kind(ErrorKind::PrematureEndOfPackedInput));
        }
        *buffer_len = input.len();
        Ok(input)
    }
}

// Unpacks 4 output bytes (half a Cap'n Proto word) according to a 4-bit tag.
//
// Each bit in the tag corresponds to one output byte (bit 0 -> byte 0, ...,
// bit 3 -> byte 3): a 1 bit copies the next byte from the packed input, and a
// 0 bit emits a zero byte. An 8-bit word tag splits into two 4-bit halves that
// both use this 16-entry table.
//
// Why only two shifts?
// Split the 4 output bytes into two adjacent pairs: bytes 0..2 and bytes 2..4.
// Within a pair, at most two bytes are present. If both are present, they are
// also adjacent in the packed input, so a single bit-shift moves the entire
// pair into position. Any absent byte in the pair is then zeroed by masking
// with `present_bytes`.
//
// Example: tag 0b1101 (bits 0, 2, 3 set) unpacks input [a, b, c, ..] into
// [a, 0, b, c]. Loaded little-endian into a u64, shifting left by 8*k moves
// bytes right by k positions in byte-array order:
//   packed:                            [a, b, c, ..]
//   low_pair  = (packed << 0) & low:   [a, b, 0, 0]   (bytes 0..2)
//   high_pair = (packed << 8) & high:  [0, 0, b, c]   (bytes 2..4)
//   low_pair | high_pair:              [a, b, b, c]
//   & present_bytes (0xffff_00ff):     [a, 0, b, c]   (clears stray b)
#[derive(Clone, Copy)]
struct UnpackHalf {
    /// Left-shift in bits for output byte pairs [0..2, 2..4].
    shifts: [u8; 2],
    /// Number of packed input bytes consumed by this half (0..=4).
    packed_len: u8,
    /// Little-endian byte mask with 0xff at each present output byte and 0x00 elsewhere.
    present_bytes: u32,
}

impl UnpackHalf {
    #[inline]
    fn unpack(&self, packed: u64) -> u64 {
        let low_pair = (packed << self.shifts[0]) & 0x0000_ffff;
        let high_pair = (packed << self.shifts[1]) & 0xffff_0000;
        (low_pair | high_pair) & u64::from(self.present_bytes)
    }
}

static UNPACK_HALVES: [UnpackHalf; 16] = {
    let mut halves = [UnpackHalf {
        shifts: [0; 2],
        packed_len: 0,
        present_bytes: 0,
    }; 16];
    let mut tag = 0;
    while tag < halves.len() {
        let entry = &mut halves[tag];
        let mut out_idx = 0u8;
        while out_idx < 4 {
            if tag & (1 << out_idx) != 0 {
                // `entry.packed_len` is this byte's index in the packed input.
                // Shifting left by `8 * (out_idx - in_idx)` moves it to `out_idx`.
                // If both bytes of a pair are present, `out_idx` and `in_idx`
                // both increment by 1, yielding the exact same shift.
                entry.shifts[(out_idx / 2) as usize] = 8 * (out_idx - entry.packed_len);
                entry.present_bytes |= 0xff << (out_idx * 8);
                entry.packed_len += 1;
            }
            out_idx += 1;
        }
        tag += 1;
    }
    halves
};

// Number of packed payload bytes (i.e. `tag.count_ones()`) for each 8-bit tag.
// Precomputed so the read loop can advance the input slice in a single lookup.
static PACKED_LENGTHS: [u8; 256] = {
    let mut lengths = [0; 256];
    let mut tag = 0;
    while tag < lengths.len() {
        lengths[tag] = UNPACK_HALVES[tag & 15].packed_len + UNPACK_HALVES[tag >> 4].packed_len;
        tag += 1;
    }
    lengths
};

#[inline]
fn unpack_word(tag: u8, bytes: &[u8; 8]) -> [u8; 8] {
    let low = &UNPACK_HALVES[usize::from(tag & 15)];
    let high = &UNPACK_HALVES[usize::from(tag >> 4)];
    let packed = u64::from_le_bytes(*bytes);
    let lower = low.unpack(packed);
    // Discard the lower half's packed bytes so the upper half's bytes start at byte 0.
    let upper = high.unpack(packed >> (8 * low.packed_len));
    (lower | (upper << 32)).to_le_bytes()
}

// On a refill error, leave the output written up to the first missing packed byte.
#[cold]
fn unpack_incomplete_word(tag: u8, packed: &[u8], word: &mut [u8; 8]) {
    let mut packed = packed.iter();
    for (bit, dest) in word.iter_mut().enumerate() {
        if tag & (1 << bit) == 0 {
            *dest = 0;
        } else if let Some(&byte) = packed.next() {
            *dest = byte;
        } else {
            break;
        }
    }
}

impl<R> Read for PackedRead<R>
where
    R: BufRead,
{
    fn read(&mut self, mut output: &mut [u8]) -> Result<usize> {
        let len = output.len();
        if len == 0 {
            return Ok(0);
        }
        assert!(
            len.is_multiple_of(8),
            "PackedRead reads must be word-aligned."
        );

        let mut input = self.inner.fill_buf()?;
        if input.is_empty() {
            return Ok(0);
        }
        let mut buffer_len = input.len();
        while let Some((word, rest)) = output.split_first_chunk_mut::<8>() {
            output = rest;
            if input.is_empty() {
                input = self.refill(&mut buffer_len)?;
            }
            let tag = input[0];
            input = &input[1..];
            if tag == 0 {
                word.fill(0);
            } else {
                let packed_len = usize::from(PACKED_LENGTHS[usize::from(tag)]);
                let mut padded = [0; 8];
                let bytes = if let Some(bytes) = input.first_chunk::<8>() {
                    input = &input[packed_len..];
                    bytes
                } else {
                    // Gather only this word's packed bytes, leaving the next tag or run count unread.
                    let mut copied = 0;
                    while packed_len - copied > input.len() {
                        padded[copied..copied + input.len()].copy_from_slice(input);
                        copied += input.len();
                        input = self.refill(&mut buffer_len).inspect_err(|_| {
                            unpack_incomplete_word(tag, &padded[..copied], word);
                        })?;
                    }
                    let (last, rest) = input.split_at(packed_len - copied);
                    padded[copied..packed_len].copy_from_slice(last);
                    input = rest;
                    &padded
                };
                *word = if tag == 0xff {
                    *bytes
                } else {
                    unpack_word(tag, bytes)
                };
            }

            if tag == 0 || tag == 0xff {
                if input.is_empty() {
                    input = self.refill(&mut buffer_len)?;
                }
                let (&count, rest) = input.split_first().unwrap();
                input = rest;
                let (run, rest) = output
                    .split_at_mut_checked(usize::from(count) * 8)
                    .ok_or_else(|| {
                        Error::from_kind(ErrorKind::PackedInputDidNotEndCleanlyOnASegmentBoundary)
                    })?;
                output = rest;

                if tag == 0 {
                    run.fill(0);
                } else if let Some((literal, rest)) = input.split_at_checked(run.len()) {
                    run.copy_from_slice(literal);
                    input = rest;
                } else {
                    // Finish a literal run directly into the caller's buffer.
                    let (buffered, remaining) = run.split_at_mut(input.len());
                    buffered.copy_from_slice(input);
                    self.inner.consume(buffer_len);
                    self.inner.read_exact(remaining)?;
                    if output.is_empty() {
                        return Ok(len);
                    }
                    input = self.inner.fill_buf()?;
                    buffer_len = input.len();
                    if input.is_empty() {
                        return Err(Error::from_kind(ErrorKind::PrematureEndOfPackedInput));
                    }
                }
            }
        }

        let consumed = buffer_len - input.len();
        self.inner.consume(consumed);
        Ok(len)
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

/// Like read_message(), but does not allocate.
///
/// Stores the message in `buffer`. Returns a `BufferNotLargeEnough`
/// error if the buffer is not large enough.
/// ALIGNMENT: If the "unaligned" feature is enabled, then there are no alignment requirements on `buffer`.
/// Otherwise, `buffer` must be 8-byte aligned (attempts to read the message will trigger errors).
pub fn read_message_no_alloc<R>(
    read: R,
    buffer: &mut [u8],
    options: message::ReaderOptions,
) -> Result<crate::message::Reader<serialize::NoAllocBufferSegments<&[u8]>>>
where
    R: BufRead,
{
    let packed_read = PackedRead { inner: read };
    serialize::read_message_no_alloc(packed_read, buffer, options)
}

/// Like try_read_message(), but does not allocate.
///
/// Stores the message in `buffer`. Returns a `BufferNotLargeEnough`
/// error if the buffer is not large enough.
/// ALIGNMENT: If the "unaligned" feature is enabled, then there are no alignment requirements on `buffer`.
/// Otherwise, `buffer` must be 8-byte aligned (attempts to read the message will trigger errors).
pub fn try_read_message_no_alloc<R>(
    read: R,
    buffer: &mut [u8],
    options: message::ReaderOptions,
) -> Result<Option<crate::message::Reader<serialize::NoAllocBufferSegments<&[u8]>>>>
where
    R: BufRead,
{
    let packed_read = PackedRead { inner: read };
    serialize::try_read_message_no_alloc(packed_read, buffer, options)
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
    fn write_all(&mut self, mut input: &[u8]) -> Result<()> {
        assert!(input.len().is_multiple_of(8));
        let mut buf = [0u8; 64];
        let mut used = 0;
        while let Some((word, rest)) = input.split_first_chunk::<8>() {
            input = rest;
            // Reserve a tag, up to eight data bytes, and an optional run length.
            if used > buf.len() - 10 {
                self.inner.write_all(&buf[..used])?;
                used = 0;
            }
            let tag_pos = used;
            used += 1;
            let mut tag = 0u8;
            for (bit, &byte) in word.iter().enumerate() {
                buf[used] = byte;
                let present = u8::from(byte != 0);
                used += usize::from(present);
                tag |= present << bit;
            }
            buf[tag_pos] = tag;
            if tag == 0 {
                let count = input[..input.len().min(255 * 8)]
                    .chunks_exact(8)
                    .take_while(|w| *w == [0; 8])
                    .count();
                buf[used] = u8::try_from(count).unwrap();
                used += 1;
                input = &input[count * 8..];
            } else if tag == 0xff {
                // Keep writing words verbatim until packing would save space.
                let count = input[..input.len().min(255 * 8)]
                    .chunks_exact(8)
                    .take_while(|w| w.iter().filter(|&&b| b == 0).count() < 2)
                    .count();
                buf[used] = u8::try_from(count).unwrap();
                used += 1;
                let (run, rest) = input.split_at(count * 8);
                input = rest;
                self.inner.write_all(&buf[..used])?;
                used = 0;
                self.inner.write_all(run)?;
            }
        }
        self.inner.write_all(&buf[..used])?;
        Ok(())
    }
}

/// Writes a packed message to a stream.
///
/// The only source of errors from this function are `write.write_all()` calls. If you pass in
/// a writer that never returns an error, then this function will never return an error.
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
    use crate::io::{Read, Write};

    use quickcheck::{TestResult, quickcheck};

    use super::read_message;
    use crate::ErrorKind;
    use crate::message::ReaderOptions;
    use crate::serialize::test::write_message_segments;
    use crate::serialize_packed::{PackedRead, PackedWrite};

    #[test]
    pub(crate) fn premature_eof() {
        let input_bytes: &[u8] = &[];
        let mut packed_read = PackedRead { inner: input_bytes };

        let mut output_bytes: alloc::vec::Vec<u8> = vec![0; 8];
        assert!(packed_read.read_exact(&mut output_bytes[..]).is_err());
    }

    pub(crate) fn check_unpacks_to(packed: &[u8], unpacked: &[u8]) {
        let mut packed_read = PackedRead { inner: packed };

        let mut bytes: alloc::vec::Vec<u8> = vec![0; unpacked.len()];
        packed_read.read_exact(&mut bytes[..]).unwrap();

        assert!(packed_read.inner.is_empty()); // nothing left to read
        assert_eq!(bytes, unpacked);
    }

    #[cfg(feature = "std")]
    #[test]
    fn unpack_across_buffer_boundaries() {
        for tag in 0..=u8::MAX {
            let mut packed = vec![tag];
            let mut expected = [0; 8];
            for (bit, byte) in expected.iter_mut().enumerate() {
                if tag & (1 << bit) != 0 {
                    *byte = bit as u8 + 1;
                    packed.push(*byte);
                }
            }
            let mut expected = expected.to_vec();
            if tag == 0 || tag == 0xff {
                packed.push(2);
                if tag == 0 {
                    expected.extend_from_slice(&[0; 16]);
                } else {
                    packed.extend(1..=16);
                    expected.extend(1..=16);
                }
            }
            // Leave a full word of lookahead so every tag also exercises the
            // buffered decoder, including tags with only one packed byte.
            packed.extend_from_slice(&[0xff, 41, 42, 43, 44, 45, 46, 47, 48, 0]);

            for split in 0..=packed.len() {
                let (first, second) = packed.split_at(split);
                let mut reader = PackedRead {
                    inner: std::io::Read::chain(first, second),
                };
                let mut actual = vec![0xff; expected.len()];
                reader.read_exact(&mut actual).unwrap();
                assert_eq!(actual, expected, "tag {tag:#x}, split {split}");
                let mut last = [0; 8];
                reader.read_exact(&mut last).unwrap();
                assert_eq!(last, [41, 42, 43, 44, 45, 46, 47, 48]);
                assert_eq!(reader.read(&mut last).unwrap(), 0);
            }
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn truncated_literal_run_across_refills() {
        let packed = [
            0xff, 1, 2, 3, 4, 5, 6, 7, 8, 1, 9, 10, 11, 12, 13, 14, 15, 16,
        ];
        for end in 1..packed.len() {
            let mut reader = PackedRead {
                inner: std::io::BufReader::with_capacity(1, &packed[..end]),
            };
            let error = reader.read(&mut [0; 16]).unwrap_err();
            let expected = if end < 10 {
                ErrorKind::PrematureEndOfPackedInput
            } else {
                ErrorKind::FailedToFillTheWholeBuffer
            };
            assert_eq!(error.kind, expected, "truncated at byte {end}");
        }
    }

    pub(crate) fn check_packing(unpacked: &[u8], packed: &[u8]) {
        // --------
        // write

        let mut bytes: alloc::vec::Vec<u8> = vec![0; packed.len()];
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
    pub(crate) fn simple_packing() {
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

        check_packing(&[0; 8], &[0, 0]);
        check_packing(&[0; 16], &[0, 1]);
        check_packing(
            &[
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            &[0, 2],
        );
        check_packing(&[0; 258 * 8], &[0, 255, 0, 1]);
    }

    quickcheck! {
        #[cfg_attr(miri, ignore)] // miri takes a long time with quickcheck
        fn test_round_trip(segments: alloc::vec::Vec<alloc::vec::Vec<crate::Word>>) -> TestResult {
            use crate::message::ReaderSegments;
            if segments.is_empty() { return TestResult::discard(); }
            let mut buf: alloc::vec::Vec<u8> = alloc::vec::Vec::new();

            write_message_segments(&mut PackedWrite { inner: &mut buf }, &segments);
            let message = read_message(&mut &buf[..], ReaderOptions::new()).unwrap();
            let result_segments = message.into_segments();

            TestResult::from_bool(segments.iter().enumerate().all(|(i, segment)| {
                crate::Word::words_to_bytes(&segment[..]) == result_segments.get_segment(i as u32).unwrap()
            }))
        }

        #[cfg_attr(miri, ignore)] // miri takes a long time with quickcheck
        fn test_unpack(packed: alloc::vec::Vec<u8>) -> TestResult {
            let len = packed.len();
            let mut packed_read = PackedRead { inner: &packed[..] };

            let mut out_buffer: alloc::vec::Vec<u8> = vec![0; len * 8];

            let _ = packed_read.read_exact(&mut out_buffer);
            TestResult::from_bool(true)
        }
    }

    #[test]
    fn did_not_end_cleanly_on_a_segment_boundary() {
        let packed = &[0xff, 1, 2, 3, 4, 5, 6, 7, 8, 37, 1, 2];
        let mut packed_read = PackedRead { inner: &packed[..] };

        let mut bytes: alloc::vec::Vec<u8> = vec![0; 200];
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

            let mut bytes: alloc::vec::Vec<u8> = vec![0; 200];
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
    fn truncated_word_writes_only_the_decodable_prefix() {
        let mut reader = PackedRead {
            inner: &[0b0101_0100, 11][..],
        };
        let mut word = [99; 8];
        let error = reader.read(&mut word).unwrap_err();
        assert_eq!(error.kind, ErrorKind::PrematureEndOfPackedInput);
        assert_eq!(word, [0, 0, 11, 0, 99, 99, 99, 99]);
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
