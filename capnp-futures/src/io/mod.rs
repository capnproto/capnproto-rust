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
    future::{self, Future},
    io, mem,
    task::{Context, Poll},
};

use capnp::fd::{BorrowedFd, OwnedFd};

pub use read_stream::ReadStream;
pub use write_queue::{write_queue, Sender};

#[cfg(feature = "futures-io")]
pub mod futures_io;
mod read_stream;
pub mod serialize;
pub mod serialize_packed;
mod write_queue;

pub type FdReadBuf = [Option<OwnedFd>];
pub type FdWriteBuf<'a> = [BorrowedFd<'a>];

#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Count {
    pub bytes: usize,
    pub fds: usize,
}

pub trait AsyncFdRead {
    // TODO: Better buffer types here may be a good idea.
    fn poll_read_with_fds(
        &mut self,
        cx: &mut Context<'_>,
        buf: &mut [u8],
        fd_buf: &mut FdReadBuf,
    ) -> Poll<io::Result<Count>>;
}

impl<T: ?Sized + AsyncFdRead> AsyncFdRead for Box<T> {
    fn poll_read_with_fds(
        &mut self,
        cx: &mut Context<'_>,
        buf: &mut [u8],
        fd_buf: &mut FdReadBuf,
    ) -> Poll<io::Result<Count>> {
        (**self).poll_read_with_fds(cx, buf, fd_buf)
    }
}

impl<T: ?Sized + AsyncFdRead> AsyncFdRead for &mut T {
    fn poll_read_with_fds(
        &mut self,
        cx: &mut Context<'_>,
        buf: &mut [u8],
        fd_buf: &mut FdReadBuf,
    ) -> Poll<io::Result<Count>> {
        (**self).poll_read_with_fds(cx, buf, fd_buf)
    }
}

impl AsyncFdRead for &[u8] {
    fn poll_read_with_fds(
        &mut self,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
        _fd_buf: &mut FdReadBuf,
    ) -> Poll<io::Result<Count>> {
        let (chunk, rest) = self.split_at(self.len().min(buf.len()));
        buf[..chunk.len()].copy_from_slice(chunk);
        *self = rest;
        Poll::Ready(Ok(Count {
            bytes: chunk.len(),
            fds: 0,
        }))
    }
}

pub trait AsyncFdReadExt: AsyncFdRead {
    fn read_with_fds(
        &mut self,
        buf: &mut [u8],
        fd_buf: &mut FdReadBuf,
    ) -> impl Future<Output = io::Result<Count>> {
        future::poll_fn(|cx| self.poll_read_with_fds(cx, buf, fd_buf))
    }

    fn read(&mut self, buf: &mut [u8]) -> impl Future<Output = io::Result<usize>> {
        future::poll_fn(|cx| {
            self.poll_read_with_fds(cx, buf, &mut [])
                .map_ok(|count| count.bytes)
        })
    }

    fn read_exact(&mut self, mut buf: &mut [u8]) -> impl Future<Output = io::Result<()>> {
        async move {
            while !buf.is_empty() {
                let bytes = self.read(buf).await?;
                (_, buf) = mem::take(&mut buf).split_at_mut(bytes);
                if bytes == 0 {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
                }
            }
            Ok(())
        }
    }
}

impl<R: AsyncFdRead> AsyncFdReadExt for R {}

pub trait AsyncFdWrite {
    fn poll_write_with_fds(
        &mut self,
        cx: &mut Context<'_>,
        buf: &[u8],
        fd_buf: &FdWriteBuf<'_>,
    ) -> Poll<io::Result<Count>>;

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>>;
}

impl<T: ?Sized + AsyncFdWrite> AsyncFdWrite for Box<T> {
    fn poll_write_with_fds(
        &mut self,
        cx: &mut Context<'_>,
        buf: &[u8],
        fd_buf: &FdWriteBuf<'_>,
    ) -> Poll<io::Result<Count>> {
        (**self).poll_write_with_fds(cx, buf, fd_buf)
    }

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (**self).poll_flush(cx)
    }
}

impl<T: ?Sized + AsyncFdWrite> AsyncFdWrite for &mut T {
    fn poll_write_with_fds(
        &mut self,
        cx: &mut Context<'_>,
        buf: &[u8],
        fd_buf: &FdWriteBuf<'_>,
    ) -> Poll<io::Result<Count>> {
        (**self).poll_write_with_fds(cx, buf, fd_buf)
    }

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        (**self).poll_flush(cx)
    }
}

impl AsyncFdWrite for Vec<u8> {
    fn poll_write_with_fds(
        &mut self,
        _cx: &mut Context<'_>,
        buf: &[u8],
        _fd_buf: &FdWriteBuf<'_>,
    ) -> Poll<io::Result<Count>> {
        self.extend(buf);
        Poll::Ready(Ok(Count {
            bytes: buf.len(),
            fds: 0,
        }))
    }

    fn poll_flush(&mut self, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

pub trait AsyncFdWriteExt: AsyncFdWrite {
    fn write_all(&mut self, buf: &[u8]) -> impl Future<Output = io::Result<()>> {
        self.write_all_with_fds(buf, &[])
    }

    fn write_all_with_fds(
        &mut self,
        mut buf: &[u8],
        mut fd_buf: &FdWriteBuf<'_>,
    ) -> impl Future<Output = io::Result<()>> {
        async move {
            while !buf.is_empty() {
                let count = future::poll_fn(|cx| self.poll_write_with_fds(cx, buf, fd_buf)).await?;
                (_, buf) = mem::take(&mut buf).split_at(count.bytes);
                if count.bytes == 0 {
                    return Err(io::Error::from(io::ErrorKind::WriteZero));
                }
                // FD passing is all‐or‐nothing.
                fd_buf = &[];
            }
            Ok(())
        }
    }

    fn flush(&mut self) -> impl Future<Output = io::Result<()>> {
        future::poll_fn(|cx| self.poll_flush(cx))
    }
}

impl<W: AsyncFdWrite> AsyncFdWriteExt for W {}
