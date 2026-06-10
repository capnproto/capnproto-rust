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
    io::{self, IoSlice, IoSliceMut},
    mem::MaybeUninit,
    os::fd::OwnedFd,
    task::{ready, Context, Poll},
};

use rustix::net::{
    recvmsg, sendmsg, RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags, SendAncillaryBuffer,
    SendAncillaryMessage, SendFlags,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::UnixStream,
};

use crate::io::{tokio::Compat, AsyncFdRead, AsyncFdWrite, Count, FdReadBuf, FdWriteBuf};

#[derive(Debug)]
pub struct UnixFdStream<T> {
    inner: T,
    cmsg_space: [MaybeUninit<u8>; CMSG_SPACE_LEN],
}

// We set this to 1024 to give ample room for both `SCM_RIGHTS`
// messages and any other ancillary data that might come along for
// the ride.
const MAX_FDS_PER_MESSAGE: usize = 1024;
const CMSG_SPACE_LEN: usize = rustix::cmsg_space!(ScmRights(MAX_FDS_PER_MESSAGE));

impl<T> UnixFdStream<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            cmsg_space: [MaybeUninit::uninit(); CMSG_SPACE_LEN],
        }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> From<T> for UnixFdStream<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> AsRef<T> for UnixFdStream<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T> AsMut<T> for UnixFdStream<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T> AsyncFdRead for UnixFdStream<T>
where
    T: AsyncRead + AsRef<UnixStream> + Unpin,
{
    fn poll_read_with_fds(
        &mut self,
        cx: &mut Context<'_>,
        buf: &mut [u8],
        fd_buf: &mut FdReadBuf,
    ) -> Poll<io::Result<Count>> {
        if fd_buf.is_empty() {
            return Compat::new(&mut self.inner).poll_read_with_fds(cx, buf, fd_buf);
        }

        let stream = self.inner.as_ref();
        let mut iov = [IoSliceMut::new(buf)];
        // Darwin has a nasty bug that leaks file descriptor table entries if
        // `SCM_RIGHTS` messages get truncated; see
        // <https://gist.github.com/kentonv/bc7592af98c68ba2738f4436920868dc>
        // for details. We work around it by always using a buffer big enough
        // to store the maximum number of FDs.
        //
        // On other platforms, we’d prefer to avoid accepting excess FDs into
        // our table to begin with, so we can avoid closing them.
        //
        // TODO: This defeats the attempt to reserve additional buffer space
        // for `SCM_CREDENTIALS`, etc.; compensating for that would be easy,
        // but compensating for `SCM_SECURITY` would require 255+ bytes if
        // someone decided to enable `SO_PASSSEC`. It’s unclear if there’s an
        // actually good solution here.
        let cmsg_space = if let Some(space) = cfg!(not(any(
            target_os = "macos",
            target_os = "ios",
            target_os = "tvos",
            target_os = "visionos",
            target_os = "watchos"
        )))
        .then_some(())
        .and_then(|()| {
            self.cmsg_space
                .get_mut(..rustix::cmsg_space!(ScmRights(fd_buf.len())))
        }) {
            space
        } else {
            &mut self.cmsg_space
        };
        let mut cmsg_buf = RecvAncillaryBuffer::new(cmsg_space);
        #[allow(unused_mut)]
        let mut flags = RecvFlags::DONTWAIT;
        #[allow(unused_mut)]
        let mut set_cloexec: fn(&OwnedFd) -> rustix::io::Result<()> =
            |fd| rustix::io::fcntl_setfd(fd, rustix::io::FdFlags::CLOEXEC);

        // Per <https://github.com/bytecodealliance/rustix/blob/v1.1.4/src/backend/libc/net/send_recv.rs#L71-L84>.
        #[cfg(not(any(
            target_os = "macos",
            target_os = "ios",
            target_os = "tvos",
            target_os = "visionos",
            target_os = "watchos",
            target_os = "solaris",
            target_os = "illumos",
            target_os = "aix",
            target_os = "haiku",
            target_os = "nto",
            target_os = "redox",
        )))]
        {
            flags |= RecvFlags::CMSG_CLOEXEC;
            set_cloexec = |_fd| Ok(());
        }

        let bytes = loop {
            ready!(stream.poll_read_ready(cx))?;
            let result = stream.try_io(tokio::io::Interest::READABLE, || {
                Ok(recvmsg(stream, &mut iov, &mut cmsg_buf, flags)?)
            });
            match result {
                Ok(received) => break received.bytes,
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => continue,
                Err(err) => return Poll::Ready(Err(err)),
            }
        };

        let mut fds = 0;
        let mut errs = Ok(());
        let mut fd_iter = cmsg_buf
            .drain()
            .filter_map(|cmsg| match cmsg {
                RecvAncillaryMessage::ScmRights(fds) => Some(fds),
                _ => None,
            })
            .flatten();
        for (slot, fd) in fd_buf.iter_mut().zip(&mut fd_iter) {
            errs = errs.and(set_cloexec(&fd));
            *slot = Some(fd);
            fds += 1;
        }

        // Drop excess file descriptors.
        //
        // TODO: It feels like it might be a bug in rustix that these seemingly
        // get leaked by default.
        //
        // TODO: `close(2)` can block (e.g. FUSE, NFS), and we’re in an async
        // context. I’m not sure if Tokio is very careful around this in
        // general, but it might be worth considering doing this with
        // `task::spawn_blocking` or similar. (But then, the exact same
        // consideration will apply to any consuming code downstream of this.)
        for _excess_fd in fd_iter {}

        Poll::Ready(errs.map_err(io::Error::from).and(Ok(Count { bytes, fds })))
    }
}

impl<T> AsyncFdWrite for UnixFdStream<T>
where
    T: AsyncWrite + AsRef<UnixStream> + Unpin,
{
    fn poll_write_with_fds(
        &mut self,
        cx: &mut Context<'_>,
        buf: &[u8],
        fd_buf: &FdWriteBuf<'_>,
    ) -> Poll<io::Result<Count>> {
        if fd_buf.is_empty() {
            return Compat::new(&mut self.inner).poll_write_with_fds(cx, buf, fd_buf);
        }

        let stream = self.inner.as_ref();
        let iov = [IoSlice::new(buf)];
        let mut cmsg_buf = SendAncillaryBuffer::new(&mut self.cmsg_space);
        // No operating system that I’m aware of supports sending 1024 FDs at
        // a time anyway, and limits vary between operating systems, so let’s
        // accept silent truncation if someone is trying to send an
        // unreasonable number.
        assert!(
            cmsg_buf.push(SendAncillaryMessage::ScmRights(
                fd_buf.get(..MAX_FDS_PER_MESSAGE).unwrap_or(fd_buf),
            )),
            "ancillary message should fit in buffer"
        );

        loop {
            ready!(stream.poll_write_ready(cx))?;
            let result = stream.try_io(tokio::io::Interest::WRITABLE, || {
                Ok(sendmsg(stream, &iov, &mut cmsg_buf, SendFlags::DONTWAIT)?)
            });
            match result {
                Ok(bytes) => {
                    return Poll::Ready(Ok(Count {
                        bytes,
                        fds: fd_buf.len(),
                    }))
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => continue,
                Err(err) => return Poll::Ready(Err(err)),
            }
        }
    }

    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Compat::new(&mut self.inner).poll_flush(cx)
    }
}
