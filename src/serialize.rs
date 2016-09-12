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
//! [standard stream framing](https://capnproto.org/encoding.html#serialization-over-a-stream).

use std::io::{self, Read, Write};

use message;
use {Error, Result, Word, OutputSegments};

use byteorder::{ByteOrder, LittleEndian};

/// Segments read from a single flat slice of words.
pub struct SliceSegments<'a> {
    words: &'a [Word],
    segment_slices : Vec<(usize, usize)>,
}

impl <'a> message::ReaderSegments for SliceSegments<'a> {
    fn get_segment<'b>(&'b self, id: u32) -> Option<&'b [Word]> {
        if id < self.segment_slices.len() as u32 {
            let (a, b) = self.segment_slices[id as usize];
            Some(&self.words[a..b])
        } else {
            None
        }
    }
}

/// Reads a serialized message from a slice of words.
pub fn read_message_from_words<'a>(slice: &'a [Word],
                                   options: message::ReaderOptions)
                                   -> Result<message::Reader<SliceSegments<'a>>>
{
    let bytes = ::Word::words_to_bytes(slice);
    let (segment_count, first_segment_len) = try!(parse_segment_table_first(&bytes[..8]));
    let mut segment_slices = Vec::with_capacity(segment_count as usize);
    segment_slices.push((0, first_segment_len));

    let bufsize = 4 * (segment_count as usize & !1);
    let (_, offsets) =
        try!(parse_segment_table_rest(
            &options, segment_count, first_segment_len,
            &bytes[8..(8 + bufsize)]));

    let words = ::Word::bytes_to_words(&bytes[(8 + bufsize)..]);
    Ok(message::Reader::new(SliceSegments { words: words, segment_slices: offsets }, options))
}

pub struct OwnedSegments {
    segment_slices: Vec<(usize, usize)>,
    owned_space: Vec<Word>,
}

impl ::message::ReaderSegments for OwnedSegments {
    fn get_segment<'a>(&'a self, id: u32) -> Option<&'a [Word]> {
        if id < self.segment_slices.len() as u32 {
            let (a, b) = self.segment_slices[id as usize];
            Some(&self.owned_space[a..b])
        } else {
            None
        }
    }
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

pub enum Async<T> {
    Ready(T),
    NotReady,
}

/// An in-progress read operation.
#[derive(Debug)]
pub struct AsyncRead<R> where R: ::std::io::Read {
    state: ReadState<R>,
}

#[derive(Debug)]
enum ReadState<R> where R: ::std::io::Read {
    Reading {
        read: R,
        options: message::ReaderOptions,
        inner: InnerReadState,
    },

    Empty,
}

#[derive(Debug)]
pub enum InnerReadState {
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
        segment_count: u32,
        first_segment_length: u32,
        /// The buffer being read into.
        segment_size_buf: Vec<u8>,
        /// The number of bytes read before being blocked.
        idx: usize,
    },

    /// Reading the message would block while trying to read the segments.
    Segments {
        segment_slices: Vec<(usize, usize)>,
        /// The segment buffer.
        owned_space: Vec<Word>,
        /// The number of bytes read into `owned_space` before being blocked.
        idx: usize,
    },
}

/// Reads a Cap'n Proto serialized message from a stream with the provided options.
///
/// For optimal performance, `reader` should be a buffered reader type.
pub fn read_message<R>(reader: &mut R, options: message::ReaderOptions) -> Result<message::Reader<OwnedSegments>>
    where R: Read
{
    let mut future = read_message_async(reader, options);
    match future.poll() {
        Ok(Async::Ready((_, m))) => Ok(m),
        Ok(Async::NotReady) => Err(::Error::failed("message read would block".into())),
        Err(e) => Err(e)
    }
}

/// Asynchronously reads a message from `reader`.
pub fn read_message_async<R>(reader: R, options: message::ReaderOptions) -> AsyncRead<R>
    where R: Read
{
    AsyncRead {
        state: ReadState::Reading {
            read: reader,
            options: options,
            inner: InnerReadState::SegmentTableFirst { buf: [0; 8], idx: 0 },
        }
    }
}

impl InnerReadState {
    fn read_helper<R>(&mut self, read: &mut R, options: &message::ReaderOptions)
                      -> Result<Async<(Vec<Word>, Vec<(usize, usize)>)>>
        where R: ::std::io::Read
    {
        loop {
            let next_state = match *self {
                InnerReadState::SegmentTableFirst { ref mut buf, ref mut idx } => {
                    *idx += try!(async_read_all(read, &mut buf[*idx..]));
                    if *idx < buf.len() {
                        return Ok(Async::NotReady)
                    } else {
                        let (segment_count, first_segment_length) = try!(parse_segment_table_first(buf));
                        if segment_count == 1 {
                            InnerReadState::Segments {
                                segment_slices: vec![(0, first_segment_length as usize)],
                                owned_space: ::Word::allocate_zeroed_vec(first_segment_length as usize),
                                idx: 0,
                            }
                        } else {
                            InnerReadState::SegmentTableRest {
                                segment_count: segment_count,
                                first_segment_length: first_segment_length,
                                segment_size_buf: vec![0u8; 4 * (segment_count as usize & !1)],
                                idx: 0,
                            }
                        }
                    }
                }

                InnerReadState::SegmentTableRest {
                    segment_count,
                    first_segment_length,
                    ref mut segment_size_buf,
                    ref mut idx,
                } => {
                    *idx += try!(async_read_all(read, &mut segment_size_buf[*idx..]));
                    if *idx < segment_size_buf.len() {
                        return Ok(Async::NotReady)
                    } else {
                        let (word_count, segment_slices) =
                            try!(parse_segment_table_rest(
                                options, segment_count, first_segment_length, segment_size_buf));
                        InnerReadState::Segments {
                            segment_slices: segment_slices,
                            owned_space: ::Word::allocate_zeroed_vec(word_count),
                            idx: 0,
                        }
                    }
                }

                InnerReadState::Segments { ref mut segment_slices, ref mut owned_space, ref mut idx } => {
                    let len = {
                        let mut bytes = ::Word::words_to_bytes_mut(owned_space);
                        *idx += try!(async_read_all(read, &mut bytes[*idx..]));
                        bytes.len()
                    };
                    if *idx < len {
                        return Ok(Async::NotReady)
                    } else {
                        let words = ::std::mem::replace(owned_space, Vec::new());
                        let slices = ::std::mem::replace(segment_slices, Vec::new());
                        return Ok(Async::Ready((words, slices)))
                    }
                }
            };

            *self = next_state;
        }
    }
}

impl <R> AsyncRead<R> where R: ::std::io::Read {
    /// Reads a Cap'n Proto serialized message from `read` with the provided options.
    pub fn poll(&mut self) -> Result<Async<(R, message::Reader<OwnedSegments>)>> {
        let (words, slices) = match &mut self.state {
            &mut ReadState::Empty => {
                return Err(::Error::failed("tried to read empty ReadState".to_string()))
            }
            &mut ReadState::Reading { ref mut read, ref options, ref mut inner } => {
                match try!(inner.read_helper(read, options)) {
                    Async::NotReady => return Ok(Async::NotReady),
                    Async::Ready(v) => v,
                }
            }
        };

        let old_self = ::std::mem::replace(&mut self.state, ReadState::Empty);
        match old_self {
            ReadState::Empty => unreachable!(),
            ReadState::Reading { read, options, ..} => {
                return Ok(Async::Ready((
                    read,
                    message::Reader::new(
                        OwnedSegments {
                            segment_slices: slices,
                            owned_space: words,
                        },
                        options))))
            }
        }
    }
}

#[cfg(feature="futures")]
impl <R> ::futures::Future for AsyncRead<R> where R: ::std::io::Read {
    type Item = (R, message::Reader<OwnedSegments>);
    type Error = Error;
    fn poll(&mut self) -> ::futures::Poll<(R, message::Reader<OwnedSegments>), Error> {
        match try!(AsyncRead::poll(self)) {
            Async::Ready(v) => Ok(::futures::Async::Ready(v)),
            Async::NotReady => Ok(::futures::Async::NotReady),
        }
    }
}

/// Reads or continues reading the first word of a segment table from `read`.
///
/// Returns the segment count and first segment length, or a state if the
/// read would block.
///
/// The segment table format for streams is defined in the Cap'n Proto
/// [encoding spec](https://capnproto.org/encoding.html#serialization-over-a-stream)
fn parse_segment_table_first(buf: &[u8]) -> Result<(u32, u32)>
{
    let segment_count = <LittleEndian as ByteOrder>::read_u32(&buf[0..4])
                                                   .wrapping_add(1);
    if segment_count >= 512 {
        return Err(Error::failed(format!("Too many segments: {}", segment_count)))
    } else if segment_count == 0 {
        return Err(Error::failed(format!("Too few segments: {}", segment_count)))
    }

    let first_segment_len = <LittleEndian as ByteOrder>::read_u32(&buf[4..8]);
    Ok((segment_count, first_segment_len))
}

/// Reads or continues reading the remaining words (after the first) of a
/// segment table from `read`.
///
/// Returns the total segment words and segment slices, or a state if the
/// read would block.
fn parse_segment_table_rest(options: &message::ReaderOptions,
                            segment_count: u32,
                            first_segment_length: u32,
                            buf: &[u8]) -> Result<(usize, Vec<(usize, usize)>)> {
    let mut total_words = first_segment_length as usize;
    let mut segment_slices = vec![(0usize, first_segment_length as usize)];

    for idx in 0..(segment_count as usize - 1) {
        let segment_len = <LittleEndian as ByteOrder>::read_u32(&buf[(idx * 4)..(idx * 4 + 4)]) as usize;
        segment_slices.push((total_words, total_words + segment_len));
        total_words += segment_len;
    }

    // Don't accept a message which the receiver couldn't possibly traverse without hitting the
    // traversal limit. Without this check, a malicious client could transmit a very large segment
    // size to make the receiver allocate excessive space and possibly crash.
    if total_words as u64 > options.traversal_limit_in_words  {
        return Err(Error::failed(
            format!("Message has {} words, which is too large. To increase the limit on the \
             receiving end, see capnp::message::ReaderOptions.", total_words)))
    }

    Ok((total_words, segment_slices))
}

/// An in-progress write operation.
#[derive(Debug)]
pub struct AsyncWrite<W, M> where W: ::std::io::Write, M: GetOutputSegments {
    state: WriteState<W, M>,
}

#[derive(Debug)]
enum WriteState<W, M> where W: ::std::io::Write, M: GetOutputSegments, {
    Writing {
        writer: W,
        message: M,
        inner: InnerWriteState,
    },
    Empty,
}

fn construct_segment_table(segments: &[&[Word]]) -> Vec<u8> {
    let mut buf = vec![0u8; (((segments.len() + 2) & !1) * 4)];
    <LittleEndian as ByteOrder>::write_u32(&mut buf[0..4], segments.len() as u32 - 1);
    for idx in 0..segments.len() {
        <LittleEndian as ByteOrder>::write_u32(
            &mut buf[(idx + 1) * 4 .. (idx + 2) * 4],
            segments[idx].len() as u32);
    }
    buf
}

#[derive(Debug)]
enum InnerWriteState {
    /// Writing the message would block while trying to write the segment table.
    OneWordSegmentTable {
        buf: [u8; 8],
        idx: usize,
    },

    MoreThanOneWordSegmentTable {
        buf: Vec<u8>,
        idx: usize,
    },

    /// Writing the message would block while trying to write the message segments.
    Segments {
        /// The next segment to write.
        segment_idx: usize,
        /// The byte offset into the next segment to write.
        idx: usize,
    },
}

impl InnerWriteState {
    fn write_helper<W, M>(&mut self, writer: &mut W, message: &mut M)
                          -> io::Result<Async<()>>
        where W: ::std::io::Write, M: GetOutputSegments,
    {
        loop {
            let new_state = match *self {
                InnerWriteState::OneWordSegmentTable { ref mut buf, ref mut idx } => {
                    *idx += try!(async_write_all(writer, &buf[*idx..]));
                    if *idx < 8 {
                        return Ok(Async::NotReady)
                    } else {
                        InnerWriteState::Segments { segment_idx: 0, idx: 0 }
                    }
                }
                InnerWriteState::MoreThanOneWordSegmentTable { ref mut buf, ref mut idx } => {
                    *idx += try!(async_write_all(writer, &buf[*idx..]));
                    if *idx < buf.len() {
                        return Ok(Async::NotReady)
                    } else {
                        InnerWriteState::Segments { segment_idx: 0, idx: 0 }
                    }
                }
                InnerWriteState::Segments { ref mut segment_idx, ref mut idx } => {
                    let segments = &*message.get_output_segments();
                    while *segment_idx < segments.len() {
                        let segment = segments[*segment_idx];
                        let buf = ::Word::words_to_bytes(segment);
                        *idx += try!(async_write_all(writer, &buf[*idx..]));
                        if *idx < buf.len() {
                            return Ok(Async::NotReady)
                        } else {
                            *segment_idx += 1;
                            *idx = 0;
                        }
                    }

                    return Ok(Async::Ready(()))
                }
            };

            *self = new_state;
        }
    }
}

#[cfg(feature="futures")]
impl <W, M> ::futures::Future for AsyncWrite<W, M> where W: ::std::io::Write, M: GetOutputSegments {
    type Item = (W, M);
    type Error = Error;
    fn poll(&mut self) -> ::futures::Poll<(W, M), Error> {
        match try!(AsyncWrite::poll(self)) {
            Async::Ready(v) => Ok(::futures::Async::Ready(v)),
            Async::NotReady => Ok(::futures::Async::NotReady),
        }
    }
}

/// Constructs a flat vector containing the entire message.
pub fn write_message_to_words<A>(message: &message::Builder<A>) -> Vec<Word>
    where A: message::Allocator
{
    let word_count = compute_serialized_size_in_words(message);
    let mut result = ::Word::allocate_zeroed_vec(word_count);
    {
        let mut bytes = ::Word::words_to_bytes_mut(&mut result[..]);
        write_message(&mut bytes, message).unwrap();
    }

    result
}

/// Something that contains segments ready to be written out.
pub trait GetOutputSegments {
    fn get_output_segments<'a>(&'a self) -> OutputSegments<'a>;
}

impl <A> GetOutputSegments for message::Builder<A> where A: message::Allocator {
    fn get_output_segments<'a>(&'a self) -> OutputSegments<'a> {
        self.get_segments_for_output()
    }
}

impl <'a, A> GetOutputSegments for &'a message::Builder<A> where A: message::Allocator {
    fn get_output_segments<'b>(&'b self) -> OutputSegments<'b> {
        self.get_segments_for_output()
    }
}

/// Writes the provided message to `writer`.
///
/// For optimal performance, `writer` should be a buffered writer. `flush` will not be called on
/// the writer.
pub fn write_message<W, A>(writer: &mut W, message: &message::Builder<A>) -> io::Result<()>
      where W: Write, A: message::Allocator
{
    let mut future = write_message_async(writer, message);
    match future.poll() {
        Ok(Async::Ready((_, _))) => Ok(()),
        Ok(Async::NotReady) => Err(io::Error::new(io::ErrorKind::Other, "message read would block")),
        Err(e) => Err(e)
    }
}

/// Begins an asynchronous write of provided message to `writer`.
pub fn write_message_async<W, M>(writer: W, message: M) -> AsyncWrite<W, M>
    where W: ::std::io::Write, M: GetOutputSegments
{
    let inner = {
        let segments = &*message.get_output_segments();
        if segments.len() == 1 {
            let mut buf = [0; 8];
            <LittleEndian as ByteOrder>::write_u32(&mut buf[4..8], segments[0].len() as u32);
            InnerWriteState::OneWordSegmentTable { buf: buf, idx: 0 }
        } else {
            let buf = construct_segment_table(segments);
            InnerWriteState::MoreThanOneWordSegmentTable { buf: buf, idx: 0 }
        }
    };

    AsyncWrite {
        state: WriteState::Writing {
            writer: writer,
            message: message,
            inner: inner,
        }
    }
}

impl <W, M> AsyncWrite<W, M> where W: ::std::io::Write, M: GetOutputSegments {
    pub fn poll(&mut self) -> io::Result<Async<(W, M)>>
    {
        match self.state {
            WriteState::Empty => {
                return Err(io::Error::new(io::ErrorKind::Other, "tried to poll empty AsyncWrite".to_string()))
            }
            WriteState::Writing { ref mut writer, ref mut message, ref mut inner } => {
                match try!(inner.write_helper(writer, message)) {
                    Async::NotReady => return Ok(Async::NotReady),
                    Async::Ready(()) => (),
                }
            }
        };

        let old_self = ::std::mem::replace(&mut self.state, WriteState::Empty);
        match old_self {
            WriteState::Empty => unreachable!(),
            WriteState::Writing { writer, message, ..} => {
                Ok(Async::Ready((writer, message)))
            }
        }
    }
}

fn compute_serialized_size(segments: &[&[Word]]) -> usize {
    // Table size
    let mut size = (segments.len() / 2) + 1;
    for segment in &*segments {
        size += segment.len();
    }
    size
}

/// Returns the number of words required to serialize the message.
pub fn compute_serialized_size_in_words<A>(message: &::message::Builder<A>) -> usize
    where A: ::message::Allocator
{
    compute_serialized_size(&*message.get_segments_for_output())
}

#[cfg(test)]
pub mod test {

    use std::cmp;
    use std::io::{self, Cursor, Read, Write};

    use quickcheck::{quickcheck, TestResult};

    use {Word, message, Result, OutputSegments};
    use message::ReaderSegments;

    use super::{
        Async,
        GetOutputSegments,
        read_message,
        read_message_async,
        read_message_from_words,
        parse_segment_table_first,
        parse_segment_table_rest,
        compute_serialized_size,
        construct_segment_table,
        write_message_async,
    };

    pub fn read_segment_table<R>(read: &mut R,
                                 options: message::ReaderOptions)
                                 -> Result<(usize, Vec<(usize, usize)>)>
    where R: Read {
        let mut firstbuf = [0; 8];
        try!(read.read_exact(&mut firstbuf));
        let (segment_count, first_segment_len) = try!(parse_segment_table_first(&firstbuf[..]));

        let mut rest_buf = vec![0; 4*(segment_count as usize & !1)];
        try!(read.read_exact(&mut rest_buf));

        parse_segment_table_rest(&options, segment_count, first_segment_len, &rest_buf[..])
    }

    /// Writes segments as if they were a Capnproto message.
    pub fn write_message_segments<W>(writer: &mut W, segments: &Vec<Vec<Word>>) where W: Write {
        let borrowed_segments: &[&[Word]] = &segments.iter()
                                                     .map(|segment| &segment[..])
                                                     .collect::<Vec<_>>()[..];

        let outbuf = construct_segment_table(borrowed_segments);
        writer.write_all(&outbuf[..]).unwrap();
        for segment in segments {
            writer.write_all(Word::words_to_bytes(segment)).unwrap()
        }
    }

    #[test]
    fn test_read_segment_table() {

        let mut buf = vec![];

        buf.extend([0,0,0,0, // 1 segments
                    0,0,0,0] // 0 length
                    .iter().cloned());
        let (words, segment_slices) = read_segment_table(&mut Cursor::new(&buf[..]),
                                                         message::ReaderOptions::new()).unwrap();
        assert_eq!(0, words);
        assert_eq!(vec![(0,0)], segment_slices);
        buf.clear();

        buf.extend([0,0,0,0, // 1 segments
                    1,0,0,0] // 1 length
                    .iter().cloned());
        let (words, segment_slices) = read_segment_table(&mut Cursor::new(&buf[..]),
                                                         message::ReaderOptions::new()).unwrap();
        assert_eq!(1, words);
        assert_eq!(vec![(0,1)], segment_slices);
        buf.clear();

        buf.extend([1,0,0,0, // 2 segments
                    1,0,0,0, // 1 length
                    1,0,0,0, // 1 length
                    0,0,0,0] // padding
                    .iter().cloned());
        let (words, segment_slices) = read_segment_table(&mut Cursor::new(&buf[..]),
                                                         message::ReaderOptions::new()).unwrap();
        assert_eq!(2, words);
        assert_eq!(vec![(0,1), (1, 2)], segment_slices);
        buf.clear();

        buf.extend([2,0,0,0, // 3 segments
                    1,0,0,0, // 1 length
                    1,0,0,0, // 1 length
                    0,1,0,0] // 256 length
                    .iter().cloned());
        let (words, segment_slices) = read_segment_table(&mut Cursor::new(&buf[..]),
                                                         message::ReaderOptions::new()).unwrap();
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
                                                         message::ReaderOptions::new()).unwrap();
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
                                   message::ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([0,0,0,0].iter().cloned()); // 1 segments
        assert!(read_segment_table(&mut Cursor::new(&buf[..]),
                                   message::ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([0,0,0,0].iter().cloned()); // 1 segments
        buf.extend([0; 3].iter().cloned());
        assert!(read_segment_table(&mut Cursor::new(&buf[..]),
                                   message::ReaderOptions::new()).is_err());
        buf.clear();

        buf.extend([255,255,255,255].iter().cloned()); // 0 segments
        assert!(read_segment_table(&mut Cursor::new(&buf[..]),
                                   message::ReaderOptions::new()).is_err());
        buf.clear();
    }

    #[test]
    fn test_construct_segment_table() {

        let segment_0 = [Word::from(0); 0];
        let segment_1 = [Word::from(1); 1];
        let segment_199 = [Word::from(199); 199];

        let buf = construct_segment_table(&[&segment_0]);
        assert_eq!(&[0,0,0,0,  // 1 segments
                     0,0,0,0], // 0 length
                   &buf[..]);

        let buf = construct_segment_table(&[&segment_1]);
        assert_eq!(&[0,0,0,0,  // 1 segments
                     1,0,0,0], // 1 length
                   &buf[..]);

        let buf = construct_segment_table(&[&segment_199]);
        assert_eq!(&[0,0,0,0,    // 1 segments
                     199,0,0,0], // 199 length
                   &buf[..]);

        let buf = construct_segment_table(&[&segment_0, &segment_1]);;
        assert_eq!(&[1,0,0,0,  // 2 segments
                     0,0,0,0,  // 0 length
                     1,0,0,0,  // 1 length
                     0,0,0,0], // padding
                   &buf[..]);

        let buf = construct_segment_table(&[&segment_199, &segment_1, &segment_199, &segment_0]);
        assert_eq!(&[3,0,0,0,   // 4 segments
                     199,0,0,0, // 199 length
                     1,0,0,0,   // 1 length
                     199,0,0,0, // 199 length
                     0,0,0,0,   // 0 length
                     0,0,0,0],  // padding
                   &buf[..]);

        let buf = construct_segment_table(
            &[&segment_199, &segment_1, &segment_199, &segment_0, &segment_1]);
        assert_eq!(&[4,0,0,0,   // 5 segments
                     199,0,0,0, // 199 length
                     1,0,0,0,   // 1 length
                     199,0,0,0, // 199 length
                     0,0,0,0,   // 0 length
                     1,0,0,0],  // 1 length
                   &buf[..]);
    }

    impl GetOutputSegments for Vec<Vec<Word>> {
        fn get_output_segments<'a>(&'a self) -> OutputSegments<'a> {
            if self.len() == 0 {
                OutputSegments::SingleSegment([&[]])
            } else if self.len() == 1 {
                OutputSegments::SingleSegment([&self[0][..]])
            } else {
                OutputSegments::MultiSegment(self.iter()
                                             .map(|segment| &segment[..])
                                             .collect::<Vec<_>>())
            }
        }
    }

    #[test]
    fn check_round_trip() {
        fn round_trip(segments: Vec<Vec<Word>>) -> TestResult {
            if segments.len() == 0 { return TestResult::discard(); }
            let mut cursor = Cursor::new(Vec::new());

            write_message_segments(&mut cursor, &segments);
            cursor.set_position(0);

            let message = read_message(&mut cursor, message::ReaderOptions::new()).unwrap();
            let result_segments = message.into_segments();

            TestResult::from_bool(segments.iter().enumerate().all(|(i, segment)| {
                &segment[..] == result_segments.get_segment(i as u32).unwrap()
            }))
        }

        quickcheck(round_trip as fn(Vec<Vec<Word>>) -> TestResult);
    }

    #[test]
    fn check_round_trip_slice_segments() {
        fn round_trip(segments: Vec<Vec<Word>>) -> TestResult {
            if segments.len() == 0 { return TestResult::discard(); }
            let (word_count, segment_table) = {
                let borrowed_segments: &[&[Word]] = &segments.iter()
                    .map(|segment| &segment[..])
                    .collect::<Vec<_>>()[..];

                (compute_serialized_size(borrowed_segments),
                 construct_segment_table(borrowed_segments))
            };

            let mut words = ::Word::allocate_zeroed_vec(word_count);
            {
                let mut bytes = ::Word::words_to_bytes_mut(&mut words[..]);
                bytes.write_all(&segment_table[..]).unwrap();
                for segment in &segments {
                    let segment_bytes = ::Word::words_to_bytes(segment);
                    bytes.write_all(segment_bytes).unwrap();
                }
            }

            let message = read_message_from_words(&words[..], message::ReaderOptions::new()).unwrap();
            let result_segments = message.into_segments();

            TestResult::from_bool(segments.iter().enumerate().all(|(i, segment)| {
                &segment[..] == result_segments.get_segment(i as u32).unwrap()
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

        /// Number of bytes read since last blocking
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

    /// Wraps a `Write` instance and introduces blocking.
    struct BlockingWrite<W> where W: Write {
        /// The wrapped writer
        writer: W,

        /// Number of bytes to write before blocking
        frequency: usize,

        /// Number of bytes written since last blocking
        idx: usize,
    }

    impl <W> BlockingWrite<W> where W: Write {
        fn new(writer: W, frequency: usize) -> BlockingWrite<W> {
            BlockingWrite { writer: writer, frequency: frequency, idx: 0 }
        }
        fn into_writer(self) -> W {
            self.writer
        }
    }

    impl <W> Write for BlockingWrite<W> where W: Write {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if self.idx == 0 {
                self.idx = self.frequency;
                Err(io::Error::new(io::ErrorKind::WouldBlock, "BlockingWrite"))
            } else {
                let len = cmp::min(self.idx, buf.len());
                let bytes_written = try!(self.writer.write(&buf[..len]));
                self.idx -= bytes_written;
                Ok(bytes_written)
            }
        }
        fn flush(&mut self) -> io::Result<()> { Ok(()) }
    }

    #[test]
    fn check_round_trip_async() {
        fn round_trip(read_block_frequency: usize,
                      write_block_frequency: usize,
                      segments: Vec<Vec<Word>>) -> TestResult
        {
            if segments.len() == 0 || read_block_frequency == 0 || write_block_frequency == 0 {
                return TestResult::discard();
            }

            let (mut read, segments) = {
                let cursor = Cursor::new(Vec::new());
                let writer = BlockingWrite::new(cursor, write_block_frequency);
                let mut state = write_message_async(writer, segments);

                let mut result = state.poll().unwrap();
                while let Async::NotReady = result {
                    result = state.poll().unwrap();
                }

                match result {
                    Async::NotReady => unreachable!(),
                    Async::Ready((writer, m)) => {
                        let mut cursor = writer.into_writer();
                        cursor.set_position(0);
                        (BlockingRead::new(cursor, read_block_frequency), m)
                    }
                }
            };

            let message = {
                let mut state = read_message_async(&mut read, Default::default());
                let mut result = state.poll().unwrap();
                while let Async::NotReady = result {
                    result = state.poll().unwrap();
                }
                match result {
                    Async::Ready((_, m)) => m,
                    _ => unreachable!(),
                }
            };
            let message_segments = message.into_segments();

            TestResult::from_bool(segments.iter().enumerate().all(|(i, segment)| {
                &segment[..] == message_segments.get_segment(i as u32).unwrap()
            }))
        }

        quickcheck(round_trip as fn(usize, usize, Vec<Vec<Word>>) -> TestResult);
    }
}
