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

use std::io::{self, Read, Write};

use {Word, Error, Result};
use private::arena;
use message::{ReaderOptions, MessageBuilder};
use serialize::OwnedSpaceMessageReader;

use byteorder::{ByteOrder, LittleEndian};

/// Unwraps a Result<AsyncValue<T, U>, E> value into a T, or returns the error or
/// continuation if the value is not a `Complete`.
macro_rules! try_async {
    ($expr:expr) => (match $expr {
        ::std::result::Result::Ok($crate::async::AsyncValue::Complete(val)) => val,
        ::std::result::Result::Ok($crate::async::AsyncValue::Continue(continuation)) => {
            return ::std::result::Result::Ok($crate::async::AsyncValue::Continue(continuation));
        },
        ::std::result::Result::Err(err) => {
            return ::std::result::Result::Err(::std::convert::From::from(err))
        }
    })
}

/// Reads bytes from `read` into `buf` until either `buf` is full, or the read
/// would block. Returns the number of bytes read.
fn async_read_all<R>(read: &mut R, buf: &mut [u8]) -> io::Result<usize> where R: Read {
    let mut idx = 0;
    while idx < buf.len() {
        let slice = &mut buf[idx..];
        match read.read(slice) {
            Ok(n) if n == 0 => return Err(io::Error::new(io::ErrorKind::Other, "Premature EOF")),
            Ok(n) => idx += n,
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => (),
            Err(e) => return Err(e),
        }
    }
    return Ok(idx)
}

/// Writes bytes from `buf` into `write` until either all bytes are written, or
/// the write would block. Returns the number of bytes written.
fn async_write_all<W>(write: &mut W, buf: &[u8]) -> io::Result<usize> where W: Write {
    let mut idx = 0;
    while idx < buf.len() {
        let slice = &buf[idx..];
        match write.write(slice) {
            Ok(n) if n == 0 => return Err(io::Error::new(io::ErrorKind::WriteZero,
                                                         "failed to write whole buffer")),
            Ok(n) => idx += n,
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => break,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => (),
            Err(e) => return Err(e),
        }
    }
    return Ok(idx)
}

/// The value of an async operation. The operation either completed successfuly, signaled by a
/// `Complete` value, or the operation would block and needs to be continued at a later time.
#[derive(Debug)]
pub enum AsyncValue<T, U> {
    Complete(T),
    Continue(U),
}

impl <T, U> AsyncValue<T, U> {
    pub fn unwrap(self) -> T {
        match self {
            AsyncValue::Complete(val) => val,
            AsyncValue::Continue(..) => panic!("called `AsyncValue::unwrap()` on a `Continue` value"),
        }
    }

    pub fn unwrap_continuation(self) -> U {
        match self {
            AsyncValue::Complete(..) => panic!("called `AsyncValue::unwrap_continuation()` on a `Complete` value"),
            AsyncValue::Continue(continuation) => continuation,
        }
    }
}

#[derive(Debug)]
pub enum ReadContinuation {

    /// Reading the message would block while trying to read the first word (the
    /// segment count, and the first segment's length).
    SegmentTableFirst {
        /// The buffer being read into.
        buf: [u8; 8],
        /// The number of bytes read before being blocked.
        idx: usize,
    },

    /// Reading the message would block while trying to read the rest of the segment table.
    SegmentTableRest {
        /// The total number of segments.
        segment_count: usize,
        /// The segment start and end offsets into the segment buffer.
        segment_slices: Vec<(usize, usize)>,
        /// The buffer being read into.
        buf: [u8; 8],
        /// The number of bytes read before being blocked.
        idx: usize,
    },

    /// Reading the message would block while trying to read the segments.
    Segments {
        /// The segment start and end offsets into the segment buffer.
        segment_slices: Vec<(usize, usize)>,
        /// The segment buffer.
        owned_space: Vec<Word>,
        /// The number of bytes read into `owned_space` before being blocked.
        idx: usize,
    },
}

#[derive(Debug)]
pub enum WriteContinuation {

    /// Writing the message would block while trying to write the segment table.
    SegmentTable {
        /// The next word of the segment table to write.
        word: usize,
        /// The byte offset into the next word to write.
        idx: usize,
    },

    /// Writing the message would block while trying to write the message segments.
    Segments {
        /// The next segment to write.
        segment: usize,
        /// The byte offset into the next segment to write.
        idx: usize,
    },
}

pub type AsyncRead = AsyncValue<OwnedSpaceMessageReader, ReadContinuation>;
pub type AsyncWrite = AsyncValue<(), WriteContinuation>;

/// Reads a Cap'n Proto serialized message from `read` with the provided options.
///
/// Takes an optional continuation value from a prior blocked message read attempt.
///
/// Returns a `Complete(OwnedSpaceMessageReader)` if the message read succeeds,
/// or a `Continue(ReadContinuation)` if the read would block. The caller should
/// use the continuation to resume reading the message again when `read` can
/// supply more bytes.
pub fn read_message_async<R>(read: &mut R,
                             options: ReaderOptions,
                             continuation: Option<ReadContinuation>)
                             -> Result<AsyncRead>
where R: Read {
    match continuation {
        None => {
            let (segment_count, first_segment_len) =
                try_async!(read_segment_table_first(read, [0; 8], 0));
            let (total_words, segment_slices) = {
                let mut segment_slices = Vec::with_capacity(segment_count);
                segment_slices.push((0, first_segment_len));
                try_async!(read_segment_table_rest(read,
                                                   options,
                                                   segment_count,
                                                   segment_slices,
                                                   [0; 8],
                                                   0))
            };
            read_segments(read,
                          options,
                          segment_slices,
                          Word::allocate_zeroed_vec(total_words),
                          0)
        },
        Some(ReadContinuation::SegmentTableFirst { buf, idx }) => {
            let (segment_count, first_segment_len) =
                try_async!(read_segment_table_first(read, buf, idx));
            let (total_words, segment_slices) = {
                let mut segment_slices = Vec::with_capacity(segment_count);
                segment_slices.push((0, first_segment_len));
                try_async!(read_segment_table_rest(read,
                                                   options,
                                                   segment_count,
                                                   segment_slices,
                                                   [0; 8],
                                                   0))
            };
            read_segments(read,
                          options,
                          segment_slices,
                          Word::allocate_zeroed_vec(total_words),
                          0)
        },
        Some(ReadContinuation::SegmentTableRest { segment_count, segment_slices, buf, idx }) => {
            let (total_words, segment_slices) =
                try_async!(read_segment_table_rest(read,
                                                   options,
                                                   segment_count,
                                                   segment_slices,
                                                   buf,
                                                   idx));
            read_segments(read,
                          options,
                          segment_slices,
                          Word::allocate_zeroed_vec(total_words),
                          0)
        },
        Some(ReadContinuation::Segments { segment_slices, owned_space, idx }) => {
            read_segments(read, options, segment_slices, owned_space, idx)
        }
    }
}

/// Reads or continues reading the first word of a segment table from `read`.
///
/// Returns the segment count and first segment length, or a continuation if the
/// read would block.
fn read_segment_table_first<R>(read: &mut R,
                               mut buf: [u8; 8],
                               mut idx: usize)
                               -> Result<AsyncValue<(usize, usize), ReadContinuation>>
where R: Read {
    idx += try!(async_read_all(read, &mut buf[idx..]));
    if idx < buf.len() {
        let continuation = ReadContinuation::SegmentTableFirst {
            buf: buf,
            idx: idx,
        };
        return Ok(AsyncValue::Continue(continuation));
    }

    let segment_count = <LittleEndian as ByteOrder>::read_u32(&buf[0..4])
                                                   .wrapping_add(1) as usize;
    if segment_count >= 512 {
        return Err(Error::new_decode_error("Too many segments.",
                                           Some(format!("{}", segment_count))));
    } else if segment_count == 0 {
        return Err(Error::new_decode_error("Too few segments.",
                                           Some(format!("{}", segment_count))));
    }

    let first_segment_len = <LittleEndian as ByteOrder>::read_u32(&buf[4..8]) as usize;
    Ok(AsyncValue::Complete((segment_count, first_segment_len)))
}

/// Reads or continues reading the remaining words (after the first) of a
/// segment table from `read`.
///
/// Returns the total segment words and segment slices, or a continuation if the
/// read would block.
///
/// `segment_slices` must contain at least the first segment.
fn read_segment_table_rest<R>(read: &mut R,
                              options: ReaderOptions,
                              segment_count: usize,
                              mut segment_slices: Vec<(usize, usize)>,
                              mut buf: [u8; 8],
                              mut idx: usize)
                              -> Result<AsyncValue<(usize, Vec<(usize, usize)>), ReadContinuation>>
where R: Read {
    let mut total_words = segment_slices[segment_slices.len() - 1].1;

    if segment_count > 1 {
        for _ in 0..((segment_count - segment_slices.len()) / 2) {
            // read two segment lengths at a time starting with the second
            // segment through the final full Word
            idx += try!(async_read_all(read, &mut buf[idx..]));
            if idx < buf.len() {
                let continuation = ReadContinuation::SegmentTableRest {
                    segment_count: segment_count,
                    segment_slices: segment_slices,
                    buf: buf,
                    idx: idx,
                };
                return Ok(AsyncValue::Continue(continuation));
            }

            let segment_len_a = <LittleEndian as ByteOrder>::read_u32(&buf[0..4]) as usize;
            let segment_len_b = <LittleEndian as ByteOrder>::read_u32(&buf[4..8]) as usize;

            segment_slices.push((total_words, total_words + segment_len_a));
            total_words += segment_len_a;
            segment_slices.push((total_words, total_words + segment_len_b));
            total_words += segment_len_b;
            idx = 0;
        }

        if segment_count % 2 == 0 {
            // read the final Word containing the last segment length and padding
            idx += try!(async_read_all(read, &mut buf[idx..]));
            if idx < buf.len() {
                let continuation = ReadContinuation::SegmentTableRest {
                    segment_count: segment_count,
                    segment_slices: segment_slices,
                    buf: buf,
                    idx: idx,
                };
                return Ok(AsyncValue::Continue(continuation));
            }

            let segment_len = <LittleEndian as ByteOrder>::read_u32(&buf[0..4]) as usize;
            segment_slices.push((total_words, total_words + segment_len));
            total_words += segment_len;
        }
    }

    // Don't accept a message which the receiver couldn't possibly traverse without hitting the
    // traversal limit. Without this check, a malicious client could transmit a very large segment
    // size to make the receiver allocate excessive space and possibly crash.
    if total_words as u64 > options.traversal_limit_in_words  {
        return Err(Error::new_decode_error(
            "Message is too large. To increase the limit on the \
             receiving end, see capnp::ReaderOptions.", Some(format!("{}", total_words))));
    }

    Ok(AsyncValue::Complete((total_words, segment_slices)))
}

/// Reads or continues reading message segments from `read`.
fn read_segments<R>(read: &mut R,
                    options: ReaderOptions,
                    segment_slices: Vec<(usize, usize)>,
                    mut owned_space: Vec<Word>,
                    mut idx: usize)
                    -> Result<AsyncRead>
where R: Read {
    {
        let buf = Word::words_to_bytes_mut(&mut owned_space[..]);
        idx += try!(async_read_all(read, &mut buf[idx..]));
    }
    if idx < owned_space.len() * 8 {
        let continuation = ReadContinuation::Segments {
            segment_slices: segment_slices,
            owned_space: owned_space,
            idx: idx,
        };
        return Ok(AsyncValue::Continue(continuation));
    }

    let arena = {
        let segments = segment_slices.iter()
                                     .map(|&(start, end)| &owned_space[start..end])
                                     .collect::<Vec<_>>();

        arena::ReaderArena::new(&segments[..], options)
    };

    let msg = OwnedSpaceMessageReader {
        options: options,
        arena: arena,
        segment_slices: segment_slices,
        owned_space: owned_space,
    };

    Ok(AsyncValue::Complete(msg))
}

/// Writes a Cap'n Proto message to `write` with the provided options.
///
/// Takes an optional continuation value from a prior blocked attempt to write
/// the message.
///
/// Returns a `Complete(())` if the message write succeeds, or a
/// `Continue(WriteContinuation)` if the write would block. The caller should
/// use the continuation to resume writing the message again when `write` can
/// take more bytes. The message *must not* be mutated in the mean time.
pub fn write_message_async<W, M>(write: &mut W,
                                 message: &mut M,
                                 continuation: Option<WriteContinuation>)
                                 -> io::Result<AsyncWrite>
where W: Write, M: MessageBuilder {
    let segments = message.get_segments_for_output();
    match continuation {
        None => {
            try_async!(write_segment_table(write, segments, 0, 0));
            write_segments(write, segments, 0, 0)
        },
        Some(WriteContinuation::SegmentTable { word, idx }) => {
            try_async!(write_segment_table(write, segments, word, idx));
            write_segments(write, segments, 0, 0)
        },
        Some(WriteContinuation::Segments { segment, idx }) => {
            write_segments(write, segments, segment, idx)
        },
    }
}

/// Writes or continues writing a segment table to `write`.
///
/// `segments` must contain at least one segment.
fn write_segment_table<W>(write: &mut W,
                          segments: &[&[Word]],
                          mut word: usize,
                          mut idx: usize)
                          -> io::Result<AsyncWrite>
where W: Write {
    let mut buf: [u8; 8] = [0; 8];
    let segment_count = segments.len();

    if word == 0 {
        // the first word contains the segment count and the first segment's length
        <LittleEndian as ByteOrder>::write_u32(&mut buf[0..4], segment_count as u32 - 1);
        <LittleEndian as ByteOrder>::write_u32(&mut buf[4..8], segments[0].len() as u32);
        idx += try!(async_write_all(write, &buf[idx..]));
        if idx != buf.len() {
            let continuation = WriteContinuation::SegmentTable {
                word: 0,
                idx: idx,
            };
            return Ok(AsyncValue::Continue(continuation));
        }
        word += 1;
        idx = 0;
    }

    if segment_count > 1 {
        for i in word..((segment_count + 1) / 2) {
            // write two segment lengths at a time starting with the second
            // segment through the final full Word
            <LittleEndian as ByteOrder>::write_u32(&mut buf[0..4], segments[i * 2 - 1].len() as u32);
            <LittleEndian as ByteOrder>::write_u32(&mut buf[4..8], segments[i * 2].len() as u32);
            idx += try!(async_write_all(write, &buf[idx..]));
            if idx != buf.len() {
                let continuation = WriteContinuation::SegmentTable {
                    word: word,
                    idx: idx,
                };
                return Ok(AsyncValue::Continue(continuation));
            }
            idx = 0;
            word += 1;
        }

        if segment_count % 2 == 0 {
            // write the final Word containing the last segment length and padding
            <LittleEndian as ByteOrder>::write_u32(&mut buf[0..4], segments[segment_count - 1].len() as u32);
            try!((&mut buf[4..8]).write_all(&[0, 0, 0, 0]));
            idx += try!(async_write_all(write, &buf[idx..]));
            if idx != buf.len() {
                let continuation = WriteContinuation::SegmentTable {
                    word: word,
                    idx: idx,
                };
                return Ok(AsyncValue::Continue(continuation));
            }
        }
    }
    Ok(AsyncValue::Complete(()))
}

/// Writes or continues writing a segment table to `write`.
///
/// `segments` must contain at least one segment.
fn write_segments<W>(write: &mut W,
                     segments: &[&[Word]],
                     segment: usize,
                     mut idx: usize) -> io::Result<AsyncWrite>
where W: Write {
    for (i, segment) in segments[segment..].iter().enumerate() {
        idx += try!(async_write_all(write, &Word::words_to_bytes(segment)[idx..]));
        if idx < segment.len() * 8 {
            let continuation = WriteContinuation::Segments {
                segment: i,
                idx: idx,
            };
            return Ok(AsyncValue::Continue(continuation));
        }
        idx = 0;
    }
    Ok(AsyncValue::Complete(()))
}

#[cfg(test)]
pub mod test {

    use std::cmp;
    use std::io::{self, Cursor, Read, Write};

    use quickcheck::{quickcheck, TestResult};

    use {Result, Word};
    use message::{MessageReader, ReaderOptions};
    use super::{
        AsyncValue,
        AsyncWrite,
        ReadContinuation,
        WriteContinuation,
        read_message_async,
        read_segment_table_first,
        read_segment_table_rest,
        write_segment_table,
        write_segments,
    };

    pub fn read_segment_table<R>(read: &mut R,
                                 options: ReaderOptions)
                                 -> Result<AsyncValue<(usize, Vec<(usize, usize)>), ReadContinuation>>
    where R: Read {
        let (segment_count, first_segment_len) = try_async!(read_segment_table_first(read, [0; 8], 0));
        let mut segment_slices = Vec::with_capacity(segment_count);
        segment_slices.push((0, first_segment_len));
        read_segment_table_rest(read, options, segment_count, segment_slices, [0; 8], 0)
    }

    /// Writes segments as if they were a Capnproto message.
    pub fn write_message_segments<W>(write: &mut W,
                                     segments: &Vec<Vec<Word>>)
                                     -> io::Result<AsyncWrite>
    where W: Write {
        let borrowed_segments: &[&[Word]] = &segments.iter()
                                                     .map(|segment| &segment[..])
                                                     .collect::<Vec<_>>()[..];
        try_async!(write_segment_table(write, borrowed_segments, 0, 0));
        write_segments(write, borrowed_segments, 0, 0)
    }

    pub fn write_message_segments_continue<W>(write: &mut W,
                                                 segments: &Vec<Vec<Word>>,
                                                 continuation: WriteContinuation)
                                                 -> io::Result<AsyncWrite>
    where W: Write {
        let borrowed_segments: &[&[Word]] = &segments.iter()
                                                     .map(|segment| &segment[..])
                                                     .collect::<Vec<_>>()[..];
        match continuation {
            WriteContinuation::SegmentTable { word, idx } => {
                try_async!(write_segment_table(write, borrowed_segments, word, idx));
                write_segments(write, borrowed_segments, 0, 0)
            },
            WriteContinuation::Segments { segment, idx } => {
                write_segments(write, borrowed_segments, segment, idx)
            },
        }
    }

    #[test]
    fn test_read_segment_table() {

        let mut buf = vec![];

        buf.extend([0,0,0,0, // 1 segments
                    0,0,0,0] // 0 length
                    .iter().cloned());
        let (words, segment_slices) = read_segment_table(&mut Cursor::new(&buf[..]),
                                                         ReaderOptions::new()).unwrap().unwrap();
        assert_eq!(0, words);
        assert_eq!(vec![(0,0)], segment_slices);
        buf.clear();

        buf.extend([0,0,0,0, // 1 segments
                    1,0,0,0] // 1 length
                    .iter().cloned());
        let (words, segment_slices) = read_segment_table(&mut Cursor::new(&buf[..]),
                                                         ReaderOptions::new()).unwrap().unwrap();
        assert_eq!(1, words);
        assert_eq!(vec![(0,1)], segment_slices);
        buf.clear();

        buf.extend([1,0,0,0, // 2 segments
                    1,0,0,0, // 1 length
                    1,0,0,0, // 1 length
                    0,0,0,0] // padding
                    .iter().cloned());
        let (words, segment_slices) = read_segment_table(&mut Cursor::new(&buf[..]),
                                                         ReaderOptions::new()).unwrap().unwrap();
        assert_eq!(2, words);
        assert_eq!(vec![(0,1), (1, 2)], segment_slices);
        buf.clear();

        buf.extend([2,0,0,0, // 3 segments
                    1,0,0,0, // 1 length
                    1,0,0,0, // 1 length
                    0,1,0,0] // 256 length
                    .iter().cloned());
        let (words, segment_slices) = read_segment_table(&mut Cursor::new(&buf[..]),
                                                         ReaderOptions::new()).unwrap().unwrap();
        assert_eq!(258, words);
        assert_eq!(vec![(0,1), (1, 2), (2, 258)], segment_slices);
        buf.clear();

        buf.extend([3,0,0,0,  // 4 segments
                    77,0,0,0, // 77 length
                    23,0,0,0, // 23 length
                    1,0,0,0,  // 1 length
                    99,0,0,0, // 99 length
                    0,0,0,0]  // padding
                    .iter().cloned());
        let (words, segment_slices) = read_segment_table(&mut Cursor::new(&buf[..]),
                                                         ReaderOptions::new()).unwrap().unwrap();
        assert_eq!(200, words);
        assert_eq!(vec![(0,77), (77, 100), (100, 101), (101, 200)], segment_slices);
        buf.clear();
    }

    #[test]
    fn test_read_invalid_segment_table() {

        let mut buf = vec![];

        buf.extend([0,2,0,0].iter().cloned()); // 513 segments
        buf.extend([0; 513 * 4].iter().cloned());
        assert!(read_segment_table(&mut Cursor::new(&buf[..]),
                                   ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([0,0,0,0].iter().cloned()); // 1 segments
        assert!(read_segment_table(&mut Cursor::new(&buf[..]),
                                   ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([0,0,0,0].iter().cloned()); // 1 segments
        buf.extend([0; 3].iter().cloned());
        assert!(read_segment_table(&mut Cursor::new(&buf[..]),
                                   ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([255,255,255,255].iter().cloned()); // 0 segments
        assert!(read_segment_table(&mut Cursor::new(&buf[..]),
                                   ReaderOptions::new()).is_err());
        buf.clear();
    }

    #[test]
    fn test_write_segment_table() {

        let mut buf = vec![];

        let segment_0 = [Word::from(0); 0];
        let segment_1 = [Word::from(1); 1];
        let segment_199 = [Word::from(199); 199];

        write_segment_table(&mut buf, &[&segment_0], 0, 0).unwrap().unwrap();
        assert_eq!(&[0,0,0,0,  // 1 segments
                     0,0,0,0], // 0 length
                   &buf[..]);
        buf.clear();

        write_segment_table(&mut buf, &[&segment_1], 0, 0).unwrap().unwrap();
        assert_eq!(&[0,0,0,0,  // 1 segments
                     1,0,0,0], // 1 length
                   &buf[..]);
        buf.clear();

        write_segment_table(&mut buf, &[&segment_199], 0, 0).unwrap().unwrap();
        assert_eq!(&[0,0,0,0,    // 1 segments
                     199,0,0,0], // 199 length
                   &buf[..]);
        buf.clear();

        write_segment_table(&mut buf, &[&segment_0, &segment_1], 0, 0).unwrap().unwrap();
        assert_eq!(&[1,0,0,0,  // 2 segments
                     0,0,0,0,  // 0 length
                     1,0,0,0,  // 1 length
                     0,0,0,0], // padding
                   &buf[..]);
        buf.clear();

        write_segment_table(&mut buf,
                            &[&segment_199, &segment_1, &segment_199, &segment_0],
                            0, 0).unwrap().unwrap();
        assert_eq!(&[3,0,0,0,   // 4 segments
                     199,0,0,0, // 199 length
                     1,0,0,0,   // 1 length
                     199,0,0,0, // 199 length
                     0,0,0,0,   // 0 length
                     0,0,0,0],  // padding
                   &buf[..]);
        buf.clear();

        write_segment_table(&mut buf,
                            &[&segment_199, &segment_1, &segment_199, &segment_0, &segment_1],
                            0, 0).unwrap().unwrap();
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
    fn check_round_trip() {
        fn round_trip(segments: Vec<Vec<Word>>) -> TestResult {
            if segments.len() == 0 { return TestResult::discard(); }
            let mut cursor = Cursor::new(Vec::new());

            write_message_segments(&mut cursor, &segments).unwrap().unwrap();
            cursor.set_position(0);

            let message =
                read_message_async(&mut cursor, ReaderOptions::new(), None).unwrap().unwrap();

            TestResult::from_bool(segments.iter().enumerate().all(|(i, segment)| {
                &segment[..] == message.get_segment(i)
            }))
        }

        quickcheck(round_trip as fn(Vec<Vec<Word>>) -> TestResult);
    }

    /// Wraps a `Read` instance and introduces blocking.
    struct BlockingRead<R> where R: Read {
        /// The wrapped reader
        read: R,

        /// Number of bytes to read before blocking
        frequency: usize,

        /// Currently read bytes after blocking
        idx: usize,
    }

    impl <R> BlockingRead<R> where R: Read {
        fn new(read: R, frequency: usize) -> BlockingRead<R> {
            BlockingRead { read: read, frequency: frequency, idx: 0 }
        }
    }

    impl <R> Read for BlockingRead<R> where R: Read {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.idx == 0 {
                self.idx = self.frequency;
                Err(io::Error::new(io::ErrorKind::WouldBlock, "BlockingRead"))
            } else {
                let len = cmp::min(self.idx, buf.len());
                let bytes_read = try!(self.read.read(&mut buf[..len]));
                self.idx -= bytes_read;
                Ok(bytes_read)
            }
        }
    }

    #[test]
    fn check_round_trip_blocking() {
        fn round_trip(frequency: usize, segments: Vec<Vec<Word>>) -> TestResult {
            if segments.len() == 0 || frequency == 0 { return TestResult::discard(); }

            let mut read = {
                let mut cursor = Cursor::new(Vec::new());
                let mut async_write = write_message_segments(&mut cursor, &segments).unwrap();
                while let AsyncValue::Continue(continuation) = async_write {
                    async_write =
                        write_message_segments_continue(&mut cursor, &segments, continuation).unwrap();
                }
                cursor.set_position(0);
                BlockingRead::new(cursor, frequency)
            };

            let message = {
                let mut msg = read_message_async(&mut read, ReaderOptions::new(), None).unwrap();
                while let AsyncValue::Continue(continuation) = msg {
                    msg = read_message_async(&mut read, ReaderOptions::new(), Some(continuation)).unwrap();
                }
                msg.unwrap()
            };

            TestResult::from_bool(segments.iter().enumerate().all(|(i, segment)| {
                &segment[..] == message.get_segment(i)
            }))
        }

        quickcheck(round_trip as fn(usize, Vec<Vec<Word>>) -> TestResult);
    }
}
