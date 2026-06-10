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

//! Asynchronous reading and writing of messages using the
//! [packed stream encoding](https://capnproto.org/encoding.html#packing).

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use capnp::serialize::OwnedSegments;
use capnp::{message, Result};
use futures_io::{AsyncRead, AsyncWrite};

use crate::io::futures_io::Compat;
use crate::io::serialize::AsOutputSegments;
use crate::io::{AsyncFdRead, AsyncFdWrite};

/// An `AsyncRead` wrapper that unpacks packed data.
pub struct PackedRead<R>
where
    R: AsyncRead + Unpin,
{
    inner: crate::io::serialize_packed::PackedRead<Compat<R>>,
}

impl<R> PackedRead<R>
where
    R: AsyncRead + Unpin,
{
    /// Creates a new `PackedRead` from a `AsyncRead`. For optimal performance,
    /// `inner` should be a buffered `AsyncRead`.
    pub fn new(inner: R) -> Self {
        Self {
            inner: crate::io::serialize_packed::PackedRead::new(Compat::new(inner)),
        }
    }
}

impl<R> AsyncRead for PackedRead<R>
where
    R: AsyncRead + Unpin,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        outbuf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.inner
            .poll_read_with_fds(cx, outbuf, &mut [])
            .map_ok(|count| count.bytes)
    }
}

/// Asynchronously reads a packed message from `read`.
///
/// Returns `None` if `read` has zero bytes left (i.e. is at end-of-file).
/// To read a stream containing an unknown number of messages, you could call
/// this function repeatedly until it returns `None`.
pub async fn try_read_message<R>(
    read: R,
    options: message::ReaderOptions,
) -> Result<Option<message::Reader<OwnedSegments>>>
where
    R: AsyncRead + Unpin,
{
    crate::io::serialize_packed::try_read_message(Compat::new(read), options).await
}

/// Asynchronously reads a message from `reader`.
pub async fn read_message<R>(
    reader: R,
    options: message::ReaderOptions,
) -> Result<message::Reader<OwnedSegments>>
where
    R: AsyncRead + Unpin,
{
    crate::io::serialize_packed::read_message(Compat::new(reader), options).await
}

/// An `AsyncWrite` wrapper that packs any data passed into it.
pub struct PackedWrite<W>
where
    W: AsyncWrite + Unpin,
{
    inner: crate::io::serialize_packed::PackedWrite<Compat<W>>,
}

/// Writes the provided message to `writer`. Does not call `writer.flush()`,
/// so that multiple successive calls can amortize work when `writer` is
/// buffered.
pub async fn write_message<W, M>(writer: W, message: M) -> Result<()>
where
    W: AsyncWrite + Unpin,
    M: AsOutputSegments,
{
    crate::io::serialize_packed::write_message(Compat::new(writer), message).await
}

impl<W> PackedWrite<W>
where
    W: AsyncWrite + Unpin,
{
    /// Creates a new `PackedWrite` from a `AsyncWrite`. For optimal performance,
    /// `inner` should be a buffered `AsyncWrite`.
    pub fn new(inner: W) -> Self {
        Self {
            inner: crate::io::serialize_packed::PackedWrite::new(Compat::new(inner)),
        }
    }
}

impl<W> AsyncWrite for PackedWrite<W>
where
    W: AsyncWrite + Unpin,
{
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        inbuf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.inner
            .poll_write_with_fds(cx, inbuf, &[])
            .map_ok(|count| count.bytes)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.inner.poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(self.inner.inner.as_mut()).poll_close(cx)
    }
}
