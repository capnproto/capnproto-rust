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
//! [standard stream framing](https://capnproto.org/encoding.html#serialization-over-a-stream),
//! where each message is preceded by a segment table indicating the size of its segments.

use alloc::vec::Vec;
use core::convert::TryInto;
use crate::io::{Read, Write};

use crate::message;
use crate::private::units::BYTES_PER_WORD;
use crate::{Error, Result};

/// Segments read from a single flat slice of words.
pub struct SliceSegments<'a> {
    words: &'a [u8],

    // Each pair represents a segment inside of `words`.
    // (starting index (in words), ending index (in words))
    segment_indices : Vec<(usize, usize)>,
}

impl <'a> message::ReaderSegments for SliceSegments<'a> {
    fn get_segment<'b>(&'b self, id: u32) -> Option<&'b [u8]> {
        if id < self.segment_indices.len() as u32 {
            let (a, b) = self.segment_indices[id as usize];
            Some(&self.words[(a * BYTES_PER_WORD)..(b * BYTES_PER_WORD)])
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.segment_indices.len()
    }
}

/// Reads a serialized message (including a segment table) from a flat slice of bytes, without copying.
/// The slice is allowed to extend beyond the end of the message. On success, updates `slice` to point
/// to the remaining bytes beyond the end of the message.
///
/// ALIGNMENT: If the "unaligned" feature is enabled, then there are no alignment requirements on `slice`.
/// Otherwise, `slice` must be 8-byte aligned (attempts to read the message will trigger errors).
pub fn read_message_from_flat_slice<'a>(slice: &mut &'a [u8],
                                        options: message::ReaderOptions)
                                        -> Result<message::Reader<SliceSegments<'a>>> {
    let all_bytes = *slice;
    let mut bytes = *slice;
    let orig_bytes_len = bytes.len();
    let segment_lengths_builder = read_segment_table(&mut bytes, options)?;
    let segment_table_bytes_len = orig_bytes_len - bytes.len();
    assert_eq!(segment_table_bytes_len % BYTES_PER_WORD, 0);
    let num_words = segment_lengths_builder.total_words();
    let body_bytes = &all_bytes[segment_table_bytes_len..];
    if  num_words > (body_bytes.len() / BYTES_PER_WORD) {
        Err(Error::failed(
            format!("Message ends prematurely. Header claimed {} words, but message only has {} words.",
                    num_words, body_bytes.len() / BYTES_PER_WORD)))
    } else {
        *slice = &body_bytes[(num_words * BYTES_PER_WORD)..];
        Ok(message::Reader::new(segment_lengths_builder.into_slice_segments(body_bytes), options))
    }
}

/// Owned memory containing a message's segments sequentialized in a single contiguous buffer.
/// The segments are guaranteed to be 8-byte aligned.
pub struct OwnedSegments {
    // Each pair represents a segment inside of `owned_space`.
    // (starting index (in words), ending index (in words))
    segment_indices : Vec<(usize, usize)>,

    owned_space: Vec<crate::Word>,
}

impl core::ops::Deref for OwnedSegments {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        crate::Word::words_to_bytes(&self.owned_space[..])
    }
}

impl core::ops::DerefMut for OwnedSegments {
    fn deref_mut(&mut self) -> &mut [u8] {
        crate::Word::words_to_bytes_mut(&mut self.owned_space[..])
    }
}

impl crate::message::ReaderSegments for OwnedSegments {
    fn get_segment<'a>(&'a self, id: u32) -> Option<&'a [u8]> {
        if id < self.segment_indices.len() as u32 {
            let (a, b) = self.segment_indices[id as usize];
            Some(&self[(a * BYTES_PER_WORD)..(b * BYTES_PER_WORD)])
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.segment_indices.len()
    }
}

/// Helper object for constructing an `OwnedSegments` or a `SliceSegments`.
pub struct SegmentLengthsBuilder {
    segment_indices: Vec<(usize, usize)>,
    total_words: usize,
}

impl SegmentLengthsBuilder {
    /// Creates a new `SegmentsLengthsBuilder`, initializing the segment_indices vector with
    /// `Vec::with_capacitiy(capacity)`. `capacity` should equal the number of times that `push_segment()`
    /// is expected to be called.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            segment_indices: Vec::with_capacity(capacity),
            total_words: 0,
        }
    }

    /// Pushes a new segment length. The `n`th time (starting at 0) this is called specifies the length of
    /// the segment with ID `n`.
    pub fn push_segment(&mut self, length_in_words: usize) {
        self.segment_indices.push((self.total_words, self.total_words + length_in_words));
        self.total_words += length_in_words;
    }

    /// Constructs an `OwnedSegments`, allocating a single buffer of 8-byte aligned memory to hold
    /// all segments.
    pub fn into_owned_segments(self) -> OwnedSegments {
        let owned_space = crate::Word::allocate_zeroed_vec(self.total_words);
        OwnedSegments {
            segment_indices: self.segment_indices,
            owned_space,
        }
    }

    /// Constructs a `SliceSegments`, where the passed-in slice is assumed to contain the segments.
    pub fn into_slice_segments(self, slice: &[u8]) -> SliceSegments {
        assert!(self.total_words * BYTES_PER_WORD <= slice.len());
        SliceSegments {
            words: slice,
            segment_indices: self.segment_indices,
        }
    }

    /// Returns the sum of the lengths of the segments pushed so far.
    pub fn total_words(&self) -> usize {
        self.total_words
    }

    /// Returns the vector of segment indices. Each entry is a pair (start_word_index, end_word_index).
    /// This method primarily exists to enable testing.
    pub fn to_segment_indices(self) -> Vec<(usize, usize)> {
        self.segment_indices
    }
}

/// Reads a serialized message from a stream with the provided options.
///
/// For optimal performance, `read` should be a buffered reader type.
pub fn read_message<R>(mut read: R, options: message::ReaderOptions) -> Result<message::Reader<OwnedSegments>>
where R: Read {
    let owned_segments_builder = read_segment_table(&mut read, options)?;
    read_segments(&mut read, owned_segments_builder.into_owned_segments(), options)
}

/// Reads a segment table from `read` and returns the total number of words across all
/// segments, as well as the segment offsets.
///
/// The segment table format for streams is defined in the Cap'n Proto
/// [encoding spec](https://capnproto.org/encoding.html)
fn read_segment_table<R>(read: &mut R,
                         options: message::ReaderOptions)
                         -> Result<SegmentLengthsBuilder>
    where R: Read
{
    let mut buf: [u8; 8] = [0; 8];

    // read the first Word, which contains segment_count and the 1st segment length
    read.read_exact(&mut buf)?;
    let segment_count = u32::from_le_bytes(buf[0..4].try_into().unwrap()).wrapping_add(1) as usize;

    if segment_count >= 512 {
        return Err(Error::failed(format!("Too many segments: {}", segment_count)))
    } else if segment_count == 0 {
        return Err(Error::failed(format!("Too few segments: {}", segment_count)))
    }

    let mut segment_lengths_builder = SegmentLengthsBuilder::with_capacity(segment_count);
    segment_lengths_builder.push_segment(u32::from_le_bytes(buf[4..8].try_into().unwrap()) as usize);
    if segment_count > 1 {
        if segment_count < 4 {
            read.read_exact(&mut buf)?;
            for idx in 0..(segment_count - 1) {
                let segment_len =
                    u32::from_le_bytes(buf[(idx * 4)..(idx + 1) * 4].try_into().unwrap()) as usize;
                segment_lengths_builder.push_segment(segment_len);
            }
        } else {
            let mut segment_sizes = vec![0u8; (segment_count & !1) * 4];
            read.read_exact(&mut segment_sizes[..])?;
            for idx in 0..(segment_count - 1) {
                let segment_len =
                    u32::from_le_bytes(segment_sizes[(idx * 4)..(idx + 1) * 4].try_into().unwrap()) as usize;
                segment_lengths_builder.push_segment(segment_len);
            }
        }
    }

    // Don't accept a message which the receiver couldn't possibly traverse without hitting the
    // traversal limit. Without this check, a malicious client could transmit a very large segment
    // size to make the receiver allocate excessive space and possibly crash.
    if segment_lengths_builder.total_words() as u64 > options.traversal_limit_in_words  {
        return Err(Error::failed(
            format!("Message has {} words, which is too large. To increase the limit on the \
             receiving end, see capnp::message::ReaderOptions.", segment_lengths_builder.total_words())))
    }

    Ok(segment_lengths_builder)
}

/// Reads segments from `read`.
fn read_segments<R>(read: &mut R,
                    mut owned_segments: OwnedSegments,
                    options: message::ReaderOptions)
                    -> Result<message::Reader<OwnedSegments>>
where R: Read {
    read.read_exact(&mut owned_segments[..])?;
    Ok(crate::message::Reader::new(owned_segments, options))
}

/// Constructs a flat vector containing the entire message.
pub fn write_message_to_words<A>(message: &message::Builder<A>) -> Vec<u8>
    where A: message::Allocator
{
    flatten_segments(&*message.get_segments_for_output())
}

pub fn write_message_segments_to_words<R>(message: &R) -> Vec<u8>
    where R: message::ReaderSegments
{
    flatten_segments(message)
}

fn flatten_segments<R: message::ReaderSegments + ?Sized>(segments: &R) -> Vec<u8> {
    let word_count = compute_serialized_size(segments);
    let segment_count = segments.len();
    let table_size = segment_count / 2 + 1;
    let mut result = Vec::with_capacity(word_count);
    for _ in 0..(table_size * BYTES_PER_WORD) {
        result.push(0);
    }
    {
        let mut bytes = &mut result[..];
        write_segment_table_internal(&mut bytes, segments).expect("Failed to write segment table.");
    }
    for i in 0..segment_count {
        let segment = segments.get_segment(i as u32).unwrap();
        for idx in 0..segment.len() {
            result.push(segment[idx]);
        }
    }
    result
}

/// Writes the provided message to `write`.
///
/// For optimal performance, `write` should be a buffered writer. `flush` will not be called on
/// the writer.
pub fn write_message<W, A>(mut write: W, message: &message::Builder<A>) -> Result<()>
 where W: Write, A: message::Allocator {
    let segments = message.get_segments_for_output();
    write_segment_table(&mut write, &segments)?;
    write_segments(&mut write, &segments)
}

pub fn write_message_segments<W, R>(mut write: W, segments: &R) -> Result<()>
 where W: Write, R: message::ReaderSegments {
    write_segment_table_internal(&mut write, segments)?;
    write_segments(&mut write, segments)
}

fn write_segment_table<W>(write: &mut W, segments: &[&[u8]]) -> Result<()>
where W: Write {
    write_segment_table_internal(write, segments)
}

/// Writes a segment table to `write`.
///
/// `segments` must contain at least one segment.
fn write_segment_table_internal<W, R>(write: &mut W, segments: &R) -> Result<()>
where W: Write, R: message::ReaderSegments + ?Sized {
    let mut buf: [u8; 8] = [0; 8];
    let segment_count = segments.len();

    // write the first Word, which contains segment_count and the 1st segment length
    buf[0..4].copy_from_slice(&(segment_count as u32 - 1).to_le_bytes());
    buf[4..8].copy_from_slice(&((segments.get_segment(0).unwrap().len() / BYTES_PER_WORD)as u32).to_le_bytes());
    write.write_all(&buf)?;

    if segment_count > 1 {
        if segment_count < 4 {
            for idx in 1..segment_count {
                buf[(idx - 1) * 4..idx * 4].copy_from_slice(
                    &((segments.get_segment(idx as u32).unwrap().len() / BYTES_PER_WORD) as u32).to_le_bytes());
            }
            if segment_count == 2 {
                for idx in 4..8 { buf[idx] = 0 }
            }
            write.write_all(&buf)?;
        } else {
            let mut buf = vec![0; (segment_count & !1) * 4];
            for idx in 1..segment_count {
                buf[(idx - 1) * 4..idx * 4].copy_from_slice(
                    &((segments.get_segment(idx as u32).unwrap().len() / BYTES_PER_WORD) as u32).to_le_bytes());
            }
            if segment_count % 2 == 0 {
                for idx in (buf.len() - 4)..(buf.len()) { buf[idx] = 0 }
            }
            write.write_all(&buf)?;
        }
    }
    Ok(())
}

/// Writes segments to `write`.
fn write_segments<W, R: message::ReaderSegments + ?Sized>(write: &mut W, segments: &R) -> Result<()>
where W: Write {
    for i in 0.. {
        if let Some(segment) = segments.get_segment(i) {
            write.write_all(segment)?;
        } else {
            break;
        }
    }
    Ok(())
}

fn compute_serialized_size<R: message::ReaderSegments + ?Sized>(segments: &R) -> usize {
    // Table size
    let len = segments.len();
    let mut size = (len / 2) + 1;
    for i in 0..len {
        let segment = segments.get_segment(i as u32).unwrap();
        size += segment.len();
    }
    size
}

/// Returns the number of words required to serialize the message.
pub fn compute_serialized_size_in_words<A>(message: &crate::message::Builder<A>) -> usize
    where A: crate::message::Allocator
{
    compute_serialized_size(&message.get_segments_for_output())
}

#[cfg(test)]
pub mod test {
    use alloc::vec::Vec;

    use crate::io::Write;

    use quickcheck::{quickcheck, TestResult};

    use crate::message;
    use crate::message::ReaderSegments;
    use super::{read_message, read_message_from_flat_slice, flatten_segments,
                read_segment_table, write_segment_table, write_segments};

    /// Writes segments as if they were a Capnproto message.
    pub fn write_message_segments<W>(write: &mut W, segments: &Vec<Vec<crate::Word>>) where W: Write {
        let borrowed_segments: &[&[u8]] = &segments.iter()
                                                   .map(|segment| crate::Word::words_to_bytes(&segment[..]))
                                                   .collect::<Vec<_>>()[..];
        write_segment_table(write, borrowed_segments).unwrap();
        write_segments(write, borrowed_segments).unwrap();
    }

    #[test]
    fn test_read_segment_table() {

        let mut buf = vec![];

        buf.extend([0,0,0,0, // 1 segments
                    0,0,0,0] // 0 length
                    .iter().cloned());
        let segment_lengths_builder = read_segment_table(&mut &buf[..],
                                                         message::ReaderOptions::new()).unwrap();
        assert_eq!(0, segment_lengths_builder.total_words());
        assert_eq!(vec![(0,0)], segment_lengths_builder.to_segment_indices());
        buf.clear();

        buf.extend([0,0,0,0, // 1 segments
                    1,0,0,0] // 1 length
                    .iter().cloned());
        let segment_lengths_builder = read_segment_table(&mut &buf[..],
                                                         message::ReaderOptions::new()).unwrap();
        assert_eq!(1, segment_lengths_builder.total_words());
        assert_eq!(vec![(0,1)], segment_lengths_builder.to_segment_indices());
        buf.clear();

        buf.extend([1,0,0,0, // 2 segments
                    1,0,0,0, // 1 length
                    1,0,0,0, // 1 length
                    0,0,0,0] // padding
                    .iter().cloned());
        let segment_lengths_builder = read_segment_table(&mut &buf[..],
                                                         message::ReaderOptions::new()).unwrap();
        assert_eq!(2, segment_lengths_builder.total_words());
        assert_eq!(vec![(0,1), (1, 2)], segment_lengths_builder.to_segment_indices());
        buf.clear();

        buf.extend([2,0,0,0, // 3 segments
                    1,0,0,0, // 1 length
                    1,0,0,0, // 1 length
                    0,1,0,0] // 256 length
                    .iter().cloned());
        let segment_lengths_builder = read_segment_table(&mut &buf[..],
                                                         message::ReaderOptions::new()).unwrap();
        assert_eq!(258, segment_lengths_builder.total_words());
        assert_eq!(vec![(0,1), (1, 2), (2, 258)], segment_lengths_builder.to_segment_indices());
        buf.clear();

        buf.extend([3,0,0,0,  // 4 segments
                    77,0,0,0, // 77 length
                    23,0,0,0, // 23 length
                    1,0,0,0,  // 1 length
                    99,0,0,0, // 99 length
                    0,0,0,0]  // padding
                    .iter().cloned());
        let segment_lengths_builder = read_segment_table(&mut &buf[..],
                                                         message::ReaderOptions::new()).unwrap();
        assert_eq!(200, segment_lengths_builder.total_words());
        assert_eq!(vec![(0,77), (77, 100), (100, 101), (101, 200)], segment_lengths_builder.to_segment_indices());
        buf.clear();
    }

    #[test]
    fn test_read_invalid_segment_table() {

        let mut buf = vec![];

        buf.extend([0,2,0,0].iter().cloned()); // 513 segments
        buf.extend([0; 513 * 4].iter().cloned());
        assert!(read_segment_table(&mut &buf[..],
                                   message::ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([0,0,0,0].iter().cloned()); // 1 segments
        assert!(read_segment_table(&mut &buf[..],
                                   message::ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([0,0,0,0].iter().cloned()); // 1 segments
        buf.extend([0; 3].iter().cloned());
        assert!(read_segment_table(&mut &buf[..],
                                   message::ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([255,255,255,255].iter().cloned()); // 0 segments
        assert!(read_segment_table(&mut &buf[..],
                                   message::ReaderOptions::new()).is_err());
        buf.clear();
    }

    #[test]
    fn test_write_segment_table() {

        let mut buf = vec![];

        let segment_0 = [0u8; 0];
        let segment_1 = [1u8,1,1,1,1,1,1,1];
        let segment_199 = [201u8; 199 * 8];

        write_segment_table(&mut buf, &[&segment_0]).unwrap();
        assert_eq!(&[0,0,0,0,  // 1 segments
                     0,0,0,0], // 0 length
                   &buf[..]);
        buf.clear();

        write_segment_table(&mut buf, &[&segment_1]).unwrap();
        assert_eq!(&[0,0,0,0,  // 1 segments
                     1,0,0,0], // 1 length
                   &buf[..]);
        buf.clear();

        write_segment_table(&mut buf, &[&segment_199]).unwrap();
        assert_eq!(&[0,0,0,0,    // 1 segments
                     199,0,0,0], // 199 length
                   &buf[..]);
        buf.clear();

        write_segment_table(&mut buf, &[&segment_0, &segment_1]).unwrap();
        assert_eq!(&[1,0,0,0,  // 2 segments
                     0,0,0,0,  // 0 length
                     1,0,0,0,  // 1 length
                     0,0,0,0], // padding
                   &buf[..]);
        buf.clear();

        write_segment_table(&mut buf,
                            &[&segment_199, &segment_1, &segment_199, &segment_0]).unwrap();
        assert_eq!(&[3,0,0,0,   // 4 segments
                     199,0,0,0, // 199 length
                     1,0,0,0,   // 1 length
                     199,0,0,0, // 199 length
                     0,0,0,0,   // 0 length
                     0,0,0,0],  // padding
                   &buf[..]);
        buf.clear();

        write_segment_table(&mut buf,
                            &[&segment_199, &segment_1, &segment_199, &segment_0, &segment_1]).unwrap();
        assert_eq!(&[4,0,0,0,   // 5 segments
                     199,0,0,0, // 199 length
                     1,0,0,0,   // 1 length
                     199,0,0,0, // 199 length
                     0,0,0,0,   // 0 length
                     1,0,0,0],  // 1 length
                   &buf[..]);
        buf.clear();
    }

    #[test]
    #[cfg_attr(miri, ignore)] // miri takes a long time with quickcheck
    fn check_round_trip() {
        fn round_trip(segments: Vec<Vec<crate::Word>>) -> TestResult {
            if segments.len() == 0 { return TestResult::discard(); }
            let mut buf: Vec<u8> = Vec::new();

            write_message_segments(&mut buf, &segments);
            let message = read_message(&mut &buf[..], message::ReaderOptions::new()).unwrap();
            let result_segments = message.into_segments();

            TestResult::from_bool(segments.iter().enumerate().all(|(i, segment)| {
                crate::Word::words_to_bytes(&segment[..]) == result_segments.get_segment(i as u32).unwrap()
            }))
        }

        quickcheck(round_trip as fn(Vec<Vec<crate::Word>>) -> TestResult);
    }

    #[test]
    #[cfg_attr(miri, ignore)] // miri takes a long time with quickcheck
    fn check_round_trip_slice_segments() {
        fn round_trip(segments: Vec<Vec<crate::Word>>) -> TestResult {
            if segments.len() == 0 { return TestResult::discard(); }
            let borrowed_segments: &[&[u8]] = &segments.iter()
                                                       .map(|segment| crate::Word::words_to_bytes(&segment[..]))
                .collect::<Vec<_>>()[..];
            let words = flatten_segments(borrowed_segments);
            let mut word_slice = &words[..];
            let message = read_message_from_flat_slice(&mut word_slice, message::ReaderOptions::new()).unwrap();
            assert!(word_slice.is_empty());  // no remaining words
            let result_segments = message.into_segments();

            TestResult::from_bool(segments.iter().enumerate().all(|(i, segment)| {
                crate::Word::words_to_bytes(&segment[..]) == result_segments.get_segment(i as u32).unwrap()
            }))
        }

        quickcheck(round_trip as fn(Vec<Vec<crate::Word>>) -> TestResult);
    }

    #[test]
    fn read_message_from_flat_slice_with_remainder() {
        let segments = vec![vec![123,0,0,0,0,0,0,0],
                            vec![4,0,0,0,0,0,0,0,
                                 5,0,0,0,0,0,0,0]];

        let borrowed_segments: &[&[u8]] = &segments.iter()
            .map(|segment| &segment[..])
            .collect::<Vec<_>>()[..];

        let mut bytes = flatten_segments(borrowed_segments);
        let extra_bytes: &[u8] = &[9,9,9,9,9,9,9,9,8,7,6,5,4,3,2,1];
        for &b in extra_bytes { bytes.push(b); }
        let mut byte_slice = &bytes[..];
        let message = read_message_from_flat_slice(&mut byte_slice, message::ReaderOptions::new()).unwrap();
        assert_eq!(byte_slice, extra_bytes);
        let result_segments = message.into_segments();
        for idx in 0..segments.len() {
            assert_eq!(
                segments[idx],
                result_segments.get_segment(idx as u32).expect("segment should exist"));
        }
    }

    #[test]
    fn read_message_from_flat_slice_too_short() {
        let segments = vec![vec![1,0,0,0,0,0,0,0],
                            vec![2,0,0,0,0,0,0,0,
                                 3,0,0,0,0,0,0,0]];

        let borrowed_segments: &[&[u8]] = &segments.iter()
            .map(|segment| &segment[..])
            .collect::<Vec<_>>()[..];

        let mut bytes = flatten_segments(borrowed_segments);
        while !bytes.is_empty() {
            bytes.pop();
            assert!(read_message_from_flat_slice(&mut &bytes[..], message::ReaderOptions::new()).is_err());
        }
    }
}
