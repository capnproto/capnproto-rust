// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors
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

//! Asynchronous reading and writing of messages using the
//! [standard stream framing](https://capnproto.org/encoding.html#serialization-over-a-stream).
//!
//! Each message is preceded by a segment table indicating the size of its segments.

use capnp::serialize::OwnedSegments;
use capnp::{message, Result};
use futures_io::{AsyncRead, AsyncWrite};

use crate::io::futures_io::Compat;
pub use crate::io::serialize::AsOutputSegments;

/// Asynchronously reads a message from `reader`.
pub async fn read_message<R>(
    reader: R,
    options: message::ReaderOptions,
) -> Result<message::Reader<OwnedSegments>>
where
    R: AsyncRead + Unpin,
{
    crate::io::serialize::read_message(Compat::new(reader), options).await
}

/// Asynchronously reads a message from `reader`.
///
/// Returns `None` if `reader` has zero bytes left (i.e. is at end-of-file).
/// To read a stream containing an unknown number of messages, you could call
/// this function repeatedly until it returns `None`.
pub async fn try_read_message<R>(
    reader: R,
    options: message::ReaderOptions,
) -> Result<Option<message::Reader<OwnedSegments>>>
where
    R: AsyncRead + Unpin,
{
    crate::io::serialize::try_read_message(Compat::new(reader), options).await
}

/// Writes the provided message to `writer`. Does not call `flush()`.
pub async fn write_message<W, M>(writer: W, message: M) -> Result<()>
where
    W: AsyncWrite + Unpin,
    M: AsOutputSegments,
{
    crate::io::serialize::write_message(Compat::new(writer), message).await
}
