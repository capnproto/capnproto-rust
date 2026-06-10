// Copyright (c) 2026 Sandstorm Development Group, Inc. and contributors
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

use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use futures_io::{AsyncRead, AsyncWrite};

pub use read_stream::ReadStream;
pub use write_queue::{write_queue, Sender};

use crate::io::{AsyncFdRead, AsyncFdWrite, Count, FdReadBuf, FdWriteBuf};

mod read_stream;
pub mod serialize;
pub mod serialize_packed;
mod write_queue;

#[derive(Debug)]
pub struct Compat<T> {
    inner: T,
}

impl<T> Compat<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> From<T> for Compat<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> AsRef<T> for Compat<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T> AsMut<T> for Compat<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<R: AsyncRead + Unpin> AsyncFdRead for Compat<R> {
    fn poll_read_with_fds(
        &mut self,
        cx: &mut Context<'_>,
        buf: &mut [u8],
        _fd_buf: &mut FdReadBuf,
    ) -> Poll<io::Result<Count>> {
        Pin::new(&mut self.inner)
            .poll_read(cx, buf)
            .map_ok(|bytes| Count { bytes, fds: 0 })
    }
}

impl<W: AsyncWrite + Unpin> AsyncFdWrite for Compat<W> {
    fn poll_write_with_fds(
        &mut self,
        cx: &mut Context<'_>,
        buf: &[u8],
        _fd_buf: &FdWriteBuf<'_>,
    ) -> Poll<io::Result<Count>> {
        Pin::new(&mut self.inner)
            .poll_write(cx, buf)
            .map_ok(|bytes| Count { bytes, fds: 0 })
    }

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }
}
