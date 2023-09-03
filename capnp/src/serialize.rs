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

mod no_alloc_buffer_segments;
pub use no_alloc_buffer_segments::{NoAllocBufferSegments, NoAllocSliceSegments};

#[cfg(feature = "alloc")]
use crate::io::{Read, Write};
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use core::convert::TryInto;
#[cfg(feature = "alloc")]
use core::ops::Deref;

use crate::message;
#[cfg(feature = "alloc")]
use crate::private::units::BYTES_PER_WORD;
use crate::Result;
#[cfg(feature = "alloc")]
use crate::{Error, ErrorKind};

pub const SEGMENTS_COUNT_LIMIT: usize = 512;

/// Segments read from a single flat slice of words.
#[cfg(feature = "alloc")]
type SliceSegments<'a> = BufferSegments<&'a [u8]>;

/// Reads a serialized message (including a segment table) from a flat slice of bytes, without copying.
/// The slice is allowed to extend beyond the end of the message. On success, updates `slice` to point
/// to the remaining bytes beyond the end of the message.
///
/// ALIGNMENT: If the "unaligned" feature is enabled, then there are no alignment requirements on `slice`.
/// Otherwise, `slice` must be 8-byte aligned (attempts to read the message will trigger errors).
#[cfg(feature = "alloc")]
pub fn read_message_from_flat_slice<'a>(
    slice: &mut &'a [u8],
    options: message::ReaderOptions,
) -> Result<message::Reader<BufferSegments<&'a [u8]>>> {
    let all_bytes = *slice;
    let mut bytes = *slice;
    let orig_bytes_len = bytes.len();
    let Some(segment_lengths_builder) = read_segment_table(&mut bytes, options)? else {
        return Err(Error::from_kind(ErrorKind::EmptySlice));
    };
    let segment_table_bytes_len = orig_bytes_len - bytes.len();
    assert_eq!(segment_table_bytes_len % BYTES_PER_WORD, 0);
    let num_words = segment_lengths_builder.total_words();
    let body_bytes = &all_bytes[segment_table_bytes_len..];
    if num_words > (body_bytes.len() / BYTES_PER_WORD) {
        Err(Error::from_kind(ErrorKind::MessageEndsPrematurely(
            num_words,
            body_bytes.len() / BYTES_PER_WORD,
        )))
    } else {
        *slice = &body_bytes[(num_words * BYTES_PER_WORD)..];
        Ok(message::Reader::new(
            segment_lengths_builder.into_slice_segments(all_bytes, segment_table_bytes_len),
            options,
        ))
    }
}

/// Reads a serialized message (including a segment table) from a flat slice of bytes, without copying.
/// The slice is allowed to extend beyond the end of the message. On success, updates `slice` to point
/// to the remaining bytes beyond the end of the message.
///
/// Unlike read_message_from_flat_slice_no_alloc it does not do heap allocation
///
/// ALIGNMENT: If the "unaligned" feature is enabled, then there are no alignment requirements on `slice`.
/// Otherwise, `slice` must be 8-byte aligned (attempts to read the message will trigger errors).
pub fn read_message_from_flat_slice_no_alloc<'a>(
    slice: &mut &'a [u8],
    options: message::ReaderOptions,
) -> Result<message::Reader<NoAllocSliceSegments<'a>>> {
    let segments = NoAllocSliceSegments::from_slice(slice, options)?;

    Ok(message::Reader::new(segments, options))
}

/// Segments read from a buffer, useful for when you have the message in a buffer and don't want the extra
/// copy of `read_message`.
#[cfg(feature = "alloc")]
pub struct BufferSegments<T> {
    buffer: T,

    // Number of bytes in the segment table.
    segment_table_bytes_len: usize,

    // Each pair represents a segment inside of `buffer`:
    // (starting index (in words), ending index (in words)),
    // where the indices are relative to the end of the segment table.
    segment_indices: Vec<(usize, usize)>,
}

#[cfg(feature = "alloc")]
impl<T: Deref<Target = [u8]>> BufferSegments<T> {
    /// Reads a serialized message (including a segment table) from a buffer and takes ownership, without copying.
    /// The buffer is allowed to be longer than the message. Provide this to `Reader::new` with options that make
    /// sense for your use case. Very long lived mmaps may need unlimited traversal limit.
    ///
    /// ALIGNMENT: If the "unaligned" feature is enabled, then there are no alignment requirements on `buffer`.
    /// Otherwise, `buffer` must be 8-byte aligned (attempts to read the message will trigger errors).
    pub fn new(buffer: T, options: message::ReaderOptions) -> Result<Self> {
        let mut segment_bytes = &*buffer;

        let Some(segment_table) = read_segment_table(&mut segment_bytes, options)? else {
            return Err(Error::from_kind(ErrorKind::EmptyBuffer));
        };
        let segment_table_bytes_len = buffer.len() - segment_bytes.len();

        assert!(segment_table.total_words() * 8 <= buffer.len());
        let segment_indices = segment_table.to_segment_indices();
        Ok(Self {
            buffer,
            segment_table_bytes_len,
            segment_indices,
        })
    }

    pub fn into_buffer(self) -> T {
        self.buffer
    }
}

#[cfg(feature = "alloc")]
impl<T: Deref<Target = [u8]>> message::ReaderSegments for BufferSegments<T> {
    fn get_segment(&self, id: u32) -> Option<&[u8]> {
        if id < self.segment_indices.len() as u32 {
            let (a, b) = self.segment_indices[id as usize];
            Some(
                &self.buffer[(self.segment_table_bytes_len + a * BYTES_PER_WORD)
                    ..(self.segment_table_bytes_len + b * BYTES_PER_WORD)],
            )
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.segment_indices.len()
    }
}

/// Owned memory containing a message's segments sequentialized in a single contiguous buffer.
/// The segments are guaranteed to be 8-byte aligned.
#[cfg(feature = "alloc")]
pub struct OwnedSegments {
    // Each pair represents a segment inside of `owned_space`.
    // (starting index (in words), ending index (in words))
    segment_indices: Vec<(usize, usize)>,

    owned_space: Vec<crate::Word>,
}

#[cfg(feature = "alloc")]
impl core::ops::Deref for OwnedSegments {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        crate::Word::words_to_bytes(&self.owned_space[..])
    }
}

#[cfg(feature = "alloc")]
impl core::ops::DerefMut for OwnedSegments {
    fn deref_mut(&mut self) -> &mut [u8] {
        crate::Word::words_to_bytes_mut(&mut self.owned_space[..])
    }
}

#[cfg(feature = "alloc")]
impl crate::message::ReaderSegments for OwnedSegments {
    fn get_segment(&self, id: u32) -> Option<&[u8]> {
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

#[cfg(feature = "alloc")]
/// Helper object for constructing an `OwnedSegments` or a `SliceSegments`.
pub struct SegmentLengthsBuilder {
    segment_indices: Vec<(usize, usize)>,
    total_words: usize,
}

#[cfg(feature = "alloc")]
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
        self.segment_indices
            .push((self.total_words, self.total_words + length_in_words));
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

    /// Constructs a `SliceSegments`.
    /// `slice` contains the full message (including the segment header).
    pub fn into_slice_segments(
        self,
        slice: &[u8],
        segment_table_bytes_len: usize,
    ) -> SliceSegments {
        assert!(self.total_words * BYTES_PER_WORD <= slice.len());
        BufferSegments {
            buffer: slice,
            segment_table_bytes_len,
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
#[cfg(feature = "alloc")]
pub fn read_message<R>(
    mut read: R,
    options: message::ReaderOptions,
) -> Result<message::Reader<OwnedSegments>>
where
    R: Read,
{
    let Some(owned_segments_builder) = read_segment_table(&mut read, options)? else {
        return Err(Error::from_kind(ErrorKind::PrematureEndOfFile));
    };
    read_segments(
        &mut read,
        owned_segments_builder.into_owned_segments(),
        options,
    )
}

/// Like `read_message()`, but returns None instead of an error if there are zero bytes left in
/// `read`. This is useful for reading a stream containing an unknown number of messages -- you
/// call this function until it returns None.
#[cfg(feature = "alloc")]
pub fn try_read_message<R>(
    mut read: R,
    options: message::ReaderOptions,
) -> Result<Option<message::Reader<OwnedSegments>>>
where
    R: Read,
{
    let Some(owned_segments_builder) = read_segment_table(&mut read, options)? else {
        return Ok(None);
    };
    Ok(Some(read_segments(
        &mut read,
        owned_segments_builder.into_owned_segments(),
        options,
    )?))
}

/// Reads a segment table from `read` and returns the total number of words across all
/// segments, as well as the segment offsets.
///
/// The segment table format for streams is defined in the Cap'n Proto
/// [encoding spec](https://capnproto.org/encoding.html)
#[cfg(feature = "alloc")]
fn read_segment_table<R>(
    read: &mut R,
    options: message::ReaderOptions,
) -> Result<Option<SegmentLengthsBuilder>>
where
    R: Read,
{
    // read the first Word, which contains segment_count and the 1st segment length
    let mut buf: [u8; 8] = [0; 8];
    {
        let n = read.read(&mut buf[..])?;
        if n == 0 {
            // Clean EOF on message boundary
            return Ok(None);
        } else if n < 8 {
            read.read_exact(&mut buf[n..])?;
        }
    }

    let segment_count = u32::from_le_bytes(buf[0..4].try_into().unwrap()).wrapping_add(1) as usize;

    if segment_count >= SEGMENTS_COUNT_LIMIT || segment_count == 0 {
        return Err(Error::from_kind(ErrorKind::InvalidNumberOfSegments(
            segment_count,
        )));
    }

    let mut segment_lengths_builder = SegmentLengthsBuilder::with_capacity(segment_count);
    segment_lengths_builder
        .push_segment(u32::from_le_bytes(buf[4..8].try_into().unwrap()) as usize);
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
                    u32::from_le_bytes(segment_sizes[(idx * 4)..(idx + 1) * 4].try_into().unwrap())
                        as usize;
                segment_lengths_builder.push_segment(segment_len);
            }
        }
    }

    // Don't accept a message which the receiver couldn't possibly traverse without hitting the
    // traversal limit. Without this check, a malicious client could transmit a very large segment
    // size to make the receiver allocate excessive space and possibly crash.
    if let Some(limit) = options.traversal_limit_in_words {
        if segment_lengths_builder.total_words() > limit {
            return Err(Error::from_kind(ErrorKind::MessageTooLarge(
                segment_lengths_builder.total_words(),
            )));
        }
    }

    Ok(Some(segment_lengths_builder))
}

#[cfg(feature = "alloc")]
/// Reads segments from `read`.
fn read_segments<R>(
    read: &mut R,
    mut owned_segments: OwnedSegments,
    options: message::ReaderOptions,
) -> Result<message::Reader<OwnedSegments>>
where
    R: Read,
{
    read.read_exact(&mut owned_segments[..])?;
    Ok(crate::message::Reader::new(owned_segments, options))
}

/// Constructs a flat vector containing the entire message, including a segment header.
#[cfg(feature = "alloc")]
pub fn write_message_to_words<A>(message: &message::Builder<A>) -> Vec<u8>
where
    A: message::Allocator,
{
    flatten_segments(&*message.get_segments_for_output())
}

/// Like `write_message_to_words()`, but takes a `ReaderSegments`, allowing it to be
/// used on `message::Reader` objects (via `into_segments()`).
#[cfg(feature = "alloc")]
pub fn write_message_segments_to_words<R>(message: &R) -> Vec<u8>
where
    R: message::ReaderSegments,
{
    flatten_segments(message)
}

#[cfg(feature = "alloc")]
fn flatten_segments<R: message::ReaderSegments + ?Sized>(segments: &R) -> Vec<u8> {
    let word_count = compute_serialized_size(segments);
    let segment_count = segments.len();
    let table_size = segment_count / 2 + 1;
    let mut result = Vec::with_capacity(word_count);
    result.resize(table_size * BYTES_PER_WORD, 0);
    {
        let mut bytes = &mut result[..];
        write_segment_table_internal(&mut bytes, segments).expect("Failed to write segment table.");
    }
    for i in 0..segment_count {
        let segment = segments.get_segment(i as u32).unwrap();
        result.extend(segment);
    }
    result
}

/// Writes the provided message to `write`.
///
/// For optimal performance, `write` should be a buffered writer. `flush()` will not be called on
/// the writer.
///
/// The only source of errors from this function are `write.write_all()` calls. If you pass in
/// a writer that never returns an error, then this function will never return an error.
#[cfg(feature = "alloc")]
pub fn write_message<W, A>(mut write: W, message: &message::Builder<A>) -> Result<()>
where
    W: Write,
    A: message::Allocator,
{
    let segments = message.get_segments_for_output();
    write_segment_table(&mut write, &segments)?;
    write_segments(&mut write, &segments)
}

/// Like `write_message()`, but takes a `ReaderSegments`, allowing it to be
/// used on `message::Reader` objects (via `into_segments()`).
#[cfg(feature = "alloc")]
pub fn write_message_segments<W, R>(mut write: W, segments: &R) -> Result<()>
where
    W: Write,
    R: message::ReaderSegments,
{
    write_segment_table_internal(&mut write, segments)?;
    write_segments(&mut write, segments)
}

#[cfg(feature = "alloc")]
fn write_segment_table<W>(write: &mut W, segments: &[&[u8]]) -> Result<()>
where
    W: Write,
{
    write_segment_table_internal(write, segments)
}

/// Writes a segment table to `write`.
///
/// `segments` must contain at least one segment.
#[cfg(feature = "alloc")]
fn write_segment_table_internal<W, R>(write: &mut W, segments: &R) -> Result<()>
where
    W: Write,
    R: message::ReaderSegments + ?Sized,
{
    let mut buf: [u8; 8] = [0; 8];
    let segment_count = segments.len();

    // write the first Word, which contains segment_count and the 1st segment length
    buf[0..4].copy_from_slice(&(segment_count as u32 - 1).to_le_bytes());
    buf[4..8].copy_from_slice(
        &((segments.get_segment(0).unwrap().len() / BYTES_PER_WORD) as u32).to_le_bytes(),
    );
    write.write_all(&buf)?;

    if segment_count > 1 {
        if segment_count < 4 {
            for idx in 1..segment_count {
                buf[(idx - 1) * 4..idx * 4].copy_from_slice(
                    &((segments.get_segment(idx as u32).unwrap().len() / BYTES_PER_WORD) as u32)
                        .to_le_bytes(),
                );
            }
            if segment_count == 2 {
                for b in &mut buf[4..8] {
                    *b = 0
                }
            }
            write.write_all(&buf)?;
        } else {
            let mut buf = vec![0; (segment_count & !1) * 4];
            for idx in 1..segment_count {
                buf[(idx - 1) * 4..idx * 4].copy_from_slice(
                    &((segments.get_segment(idx as u32).unwrap().len() / BYTES_PER_WORD) as u32)
                        .to_le_bytes(),
                );
            }
            if segment_count % 2 == 0 {
                let start_idx = buf.len() - 4;
                for b in &mut buf[start_idx..] {
                    *b = 0
                }
            }
            write.write_all(&buf)?;
        }
    }
    Ok(())
}

/// Writes segments to `write`.
#[cfg(feature = "alloc")]
fn write_segments<W, R: message::ReaderSegments + ?Sized>(write: &mut W, segments: &R) -> Result<()>
where
    W: Write,
{
    for i in 0.. {
        if let Some(segment) = segments.get_segment(i) {
            write.write_all(segment)?;
        } else {
            break;
        }
    }
    Ok(())
}

#[cfg(feature = "alloc")]
fn compute_serialized_size<R: message::ReaderSegments + ?Sized>(segments: &R) -> usize {
    // Table size
    let len = segments.len();
    let mut size = (len / 2) + 1;
    for i in 0..len {
        let segment = segments.get_segment(i as u32).unwrap();
        size += segment.len() / BYTES_PER_WORD;
    }
    size
}

/// Returns the number of (8-byte) words required to serialize the message (including the
/// segment table).
///
/// Multiply this by 8 (or `std::mem::size_of::<capnp::Word>()`) to get the number of bytes
/// that [`write_message()`](fn.write_message.html) will write.
#[cfg(feature = "alloc")]
pub fn compute_serialized_size_in_words<A>(message: &crate::message::Builder<A>) -> usize
where
    A: crate::message::Allocator,
{
    compute_serialized_size(&message.get_segments_for_output())
}

#[cfg(feature = "alloc")]
#[cfg(test)]
pub mod test {
    use alloc::vec::Vec;

    use crate::io::{Read, Write};

    use quickcheck::{quickcheck, TestResult};

    use super::{
        flatten_segments, read_message, read_message_from_flat_slice, read_segment_table,
        try_read_message, write_segment_table, write_segments,
    };
    use crate::message;
    use crate::message::ReaderSegments;

    /// Writes segments as if they were a Capnproto message.
    pub fn write_message_segments<W>(write: &mut W, segments: &[Vec<crate::Word>])
    where
        W: Write,
    {
        let borrowed_segments: &[&[u8]] = &segments
            .iter()
            .map(|segment| crate::Word::words_to_bytes(&segment[..]))
            .collect::<Vec<_>>()[..];
        write_segment_table(write, borrowed_segments).unwrap();
        write_segments(write, borrowed_segments).unwrap();
    }

    #[test]
    fn try_read_empty() {
        let mut buf: &[u8] = &[];
        assert!(try_read_message(&mut buf, message::ReaderOptions::new())
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_read_segment_table() {
        let mut buf = vec![];

        buf.extend(
            [
                0, 0, 0, 0, // 1 segments
                0, 0, 0, 0,
            ], // 0 length
        );
        let segment_lengths_builder =
            read_segment_table(&mut &buf[..], message::ReaderOptions::new())
                .unwrap()
                .unwrap();
        assert_eq!(0, segment_lengths_builder.total_words());
        assert_eq!(vec![(0, 0)], segment_lengths_builder.to_segment_indices());
        buf.clear();

        buf.extend(
            [
                0, 0, 0, 0, // 1 segments
                1, 0, 0, 0,
            ], // 1 length
        );
        let segment_lengths_builder =
            read_segment_table(&mut &buf[..], message::ReaderOptions::new())
                .unwrap()
                .unwrap();
        assert_eq!(1, segment_lengths_builder.total_words());
        assert_eq!(vec![(0, 1)], segment_lengths_builder.to_segment_indices());
        buf.clear();

        buf.extend(
            [
                1, 0, 0, 0, // 2 segments
                1, 0, 0, 0, // 1 length
                1, 0, 0, 0, // 1 length
                0, 0, 0, 0,
            ], // padding
        );
        let segment_lengths_builder =
            read_segment_table(&mut &buf[..], message::ReaderOptions::new())
                .unwrap()
                .unwrap();
        assert_eq!(2, segment_lengths_builder.total_words());
        assert_eq!(
            vec![(0, 1), (1, 2)],
            segment_lengths_builder.to_segment_indices()
        );
        buf.clear();

        buf.extend(
            [
                2, 0, 0, 0, // 3 segments
                1, 0, 0, 0, // 1 length
                1, 0, 0, 0, // 1 length
                0, 1, 0, 0,
            ], // 256 length
        );
        let segment_lengths_builder =
            read_segment_table(&mut &buf[..], message::ReaderOptions::new())
                .unwrap()
                .unwrap();
        assert_eq!(258, segment_lengths_builder.total_words());
        assert_eq!(
            vec![(0, 1), (1, 2), (2, 258)],
            segment_lengths_builder.to_segment_indices()
        );
        buf.clear();

        buf.extend(
            [
                3, 0, 0, 0, // 4 segments
                77, 0, 0, 0, // 77 length
                23, 0, 0, 0, // 23 length
                1, 0, 0, 0, // 1 length
                99, 0, 0, 0, // 99 length
                0, 0, 0, 0,
            ], // padding
        );
        let segment_lengths_builder =
            read_segment_table(&mut &buf[..], message::ReaderOptions::new())
                .unwrap()
                .unwrap();
        assert_eq!(200, segment_lengths_builder.total_words());
        assert_eq!(
            vec![(0, 77), (77, 100), (100, 101), (101, 200)],
            segment_lengths_builder.to_segment_indices()
        );
        buf.clear();
    }

    struct MaxRead<R>
    where
        R: Read,
    {
        inner: R,
        max: usize,
    }

    impl<R> Read for MaxRead<R>
    where
        R: Read,
    {
        fn read(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
            if buf.len() <= self.max {
                self.inner.read(buf)
            } else {
                self.inner.read(&mut buf[0..self.max])
            }
        }
    }

    #[test]
    fn test_read_segment_table_max_read() {
        // Make sure things still work well when we read less than a word at a time.
        let mut buf: Vec<u8> = vec![];
        buf.extend(
            [
                0, 0, 0, 0, // 1 segments
                1, 0, 0, 0,
            ], // 1 length
        );
        let segment_lengths_builder = read_segment_table(
            &mut MaxRead {
                inner: &buf[..],
                max: 2,
            },
            message::ReaderOptions::new(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(1, segment_lengths_builder.total_words());
        assert_eq!(vec![(0, 1)], segment_lengths_builder.to_segment_indices());
    }

    #[test]
    fn test_read_invalid_segment_table() {
        let mut buf = vec![];

        buf.extend([0, 2, 0, 0]); // 513 segments
        buf.extend([0; 513 * 4]);
        assert!(read_segment_table(&mut &buf[..], message::ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([0, 0, 0, 0]); // 1 segments
        assert!(read_segment_table(&mut &buf[..], message::ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([0, 0, 0, 0]); // 1 segments
        buf.extend([0; 3]);
        assert!(read_segment_table(&mut &buf[..], message::ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([255, 255, 255, 255]); // 0 segments
        assert!(read_segment_table(&mut &buf[..], message::ReaderOptions::new()).is_err());
        buf.clear();
    }

    #[test]
    fn test_write_segment_table() {
        let mut buf = vec![];

        let segment_0 = [0u8; 0];
        let segment_1 = [1u8, 1, 1, 1, 1, 1, 1, 1];
        let segment_199 = [201u8; 199 * 8];

        write_segment_table(&mut buf, &[&segment_0]).unwrap();
        assert_eq!(
            &[
                0, 0, 0, 0, // 1 segments
                0, 0, 0, 0
            ], // 0 length
            &buf[..]
        );
        buf.clear();

        write_segment_table(&mut buf, &[&segment_1]).unwrap();
        assert_eq!(
            &[
                0, 0, 0, 0, // 1 segments
                1, 0, 0, 0
            ], // 1 length
            &buf[..]
        );
        buf.clear();

        write_segment_table(&mut buf, &[&segment_199]).unwrap();
        assert_eq!(
            &[
                0, 0, 0, 0, // 1 segments
                199, 0, 0, 0
            ], // 199 length
            &buf[..]
        );
        buf.clear();

        write_segment_table(&mut buf, &[&segment_0, &segment_1]).unwrap();
        assert_eq!(
            &[
                1, 0, 0, 0, // 2 segments
                0, 0, 0, 0, // 0 length
                1, 0, 0, 0, // 1 length
                0, 0, 0, 0
            ], // padding
            &buf[..]
        );
        buf.clear();

        write_segment_table(
            &mut buf,
            &[&segment_199, &segment_1, &segment_199, &segment_0],
        )
        .unwrap();
        assert_eq!(
            &[
                3, 0, 0, 0, // 4 segments
                199, 0, 0, 0, // 199 length
                1, 0, 0, 0, // 1 length
                199, 0, 0, 0, // 199 length
                0, 0, 0, 0, // 0 length
                0, 0, 0, 0
            ], // padding
            &buf[..]
        );
        buf.clear();

        write_segment_table(
            &mut buf,
            &[
                &segment_199,
                &segment_1,
                &segment_199,
                &segment_0,
                &segment_1,
            ],
        )
        .unwrap();
        assert_eq!(
            &[
                4, 0, 0, 0, // 5 segments
                199, 0, 0, 0, // 199 length
                1, 0, 0, 0, // 1 length
                199, 0, 0, 0, // 199 length
                0, 0, 0, 0, // 0 length
                1, 0, 0, 0
            ], // 1 length
            &buf[..]
        );
        buf.clear();
    }

    quickcheck! {
        #[cfg_attr(miri, ignore)] // miri takes a long time with quickcheck
        fn test_round_trip(segments: Vec<Vec<crate::Word>>) -> TestResult {
            if segments.is_empty() { return TestResult::discard(); }
            let mut buf: Vec<u8> = Vec::new();

            write_message_segments(&mut buf, &segments);
            let message = read_message(&mut &buf[..], message::ReaderOptions::new()).unwrap();
            let result_segments = message.into_segments();

            TestResult::from_bool(segments.iter().enumerate().all(|(i, segment)| {
                crate::Word::words_to_bytes(&segment[..]) == result_segments.get_segment(i as u32).unwrap()
            }))
        }

        #[cfg_attr(miri, ignore)] // miri takes a long time with quickcheck
        fn test_round_trip_slice_segments(segments: Vec<Vec<crate::Word>>) -> TestResult {
            if segments.is_empty() { return TestResult::discard(); }
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
    }

    #[test]
    fn read_message_from_flat_slice_with_remainder() {
        let segments = vec![
            vec![123, 0, 0, 0, 0, 0, 0, 0],
            vec![4, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0],
        ];

        let borrowed_segments: &[&[u8]] = &segments
            .iter()
            .map(|segment| &segment[..])
            .collect::<Vec<_>>()[..];

        let mut bytes = flatten_segments(borrowed_segments);
        let extra_bytes: &[u8] = &[9, 9, 9, 9, 9, 9, 9, 9, 8, 7, 6, 5, 4, 3, 2, 1];
        for &b in extra_bytes {
            bytes.push(b);
        }
        let mut byte_slice = &bytes[..];
        let message =
            read_message_from_flat_slice(&mut byte_slice, message::ReaderOptions::new()).unwrap();
        assert_eq!(byte_slice, extra_bytes);
        let result_segments = message.into_segments();
        for (idx, segment) in segments.iter().enumerate() {
            assert_eq!(
                *segment,
                result_segments
                    .get_segment(idx as u32)
                    .expect("segment should exist")
            );
        }
    }

    #[test]
    fn read_message_from_flat_slice_too_short() {
        let segments = vec![
            vec![1, 0, 0, 0, 0, 0, 0, 0],
            vec![2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0],
        ];

        let borrowed_segments: &[&[u8]] = &segments
            .iter()
            .map(|segment| &segment[..])
            .collect::<Vec<_>>()[..];

        let mut bytes = flatten_segments(borrowed_segments);
        while !bytes.is_empty() {
            bytes.pop();
            assert!(
                read_message_from_flat_slice(&mut &bytes[..], message::ReaderOptions::new())
                    .is_err()
            );
        }
    }

    #[test]
    fn compute_serialized_size() {
        const LIST_LENGTH_IN_WORDS: u32 = 5;
        let mut m = message::Builder::new_default();
        {
            let root: crate::any_pointer::Builder = m.init_root();
            let _list_builder: crate::primitive_list::Builder<u64> =
                root.initn_as(LIST_LENGTH_IN_WORDS);
        }

        // The message body has a list pointer (one word) and the list (LIST_LENGTH_IN_WORDS words).
        // The message has one segment, so the header is one word.
        assert_eq!(
            super::compute_serialized_size_in_words(&m) as u32,
            1 + 1 + LIST_LENGTH_IN_WORDS
        )
    }
}
