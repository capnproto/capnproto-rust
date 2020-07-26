
use core::pin::Pin;
use core::task::Poll;
use core::task::Context;

use capnp::Result;

pub use async_ext::AsyncReadExt;
pub use async_ext::AsyncWriteExt;

pub trait AsyncRead {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8])
        -> Poll<Result<usize>>;
}

pub trait AsyncBufRead: AsyncRead {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<Result<&[u8]>>;

    fn consume(self: Pin<&mut Self>, amt: usize);
}

pub trait AsyncWrite {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8])
        -> Poll<Result<usize>>;

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>>;
}

#[cfg(feature="std")]
mod std_impls {
    use crate::async_io::{AsyncRead, AsyncBufRead, AsyncWrite};
    use capnp::Result;
    use futures::task::{Context, Poll};
    use std::pin::Pin;

    impl<R> AsyncRead for R where R: futures::AsyncRead {
        fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
            match futures::AsyncRead::poll_read(self, cx, buf) {
                Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
                Poll::Ready(Ok(n)) => Poll::Ready(Ok(n)),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    impl<R> AsyncBufRead for R where R: futures::AsyncBufRead {
        fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<&[u8]>> {
            match futures::AsyncBufRead::poll_fill_buf(self, cx) {
                Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
                Poll::Ready(Ok(buf)) => Poll::Ready(Ok(buf)),
                Poll::Pending => Poll::Pending,
            }
        }

        fn consume(self: Pin<&mut Self>, amt: usize) {
            futures::AsyncBufRead::consume(self, amt)
        }
    }

    impl<W> AsyncWrite for W where W: futures::AsyncWrite {
        fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
            match futures::AsyncWrite::poll_write(self, cx, buf) {
                Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
                Poll::Ready(Ok(n)) => Poll::Ready(Ok(n)),
                Poll::Pending => Poll::Pending,
            }
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
            match futures::AsyncWrite::poll_flush(self, cx) {
                Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                Poll::Pending => Poll::Pending,
            }
        }

        fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
            match futures::AsyncWrite::poll_close(self, cx) {
                Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

#[cfg(not(feature="std"))]
mod no_std_impls {
    use crate::async_io::{AsyncRead, AsyncBufRead, AsyncWrite};
    use core::pin::Pin;
    use core::task::{Context, Poll};
    use capnp::Result;

    impl AsyncRead for &[u8] {
        fn poll_read(mut self: Pin<&mut Self>, _: &mut Context<'_>, buf: &mut [u8])
             -> Poll<Result<usize>>
        {
            Poll::Ready(capnp::io::Read::read(&mut *self, buf))
        }
    }

    impl<R: ?Sized + AsyncRead + Unpin> AsyncRead for &mut R {
        fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize>> {
            Pin::new(&mut **self).poll_read(cx, buf)
        }
    }

    impl AsyncBufRead for &[u8] {
        fn poll_fill_buf(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<&[u8]>> {
            Poll::Ready(capnp::io::BufRead::fill_buf(self.get_mut()))
        }

        fn consume(mut self: Pin<&mut Self>, amt: usize) {
            capnp::io::BufRead::consume(&mut *self, amt)
        }
    }

    impl<R: ?Sized + AsyncBufRead + Unpin> AsyncBufRead for &mut R {
        fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<&[u8]>> {
            Pin::new(&mut **self.get_mut()).poll_fill_buf(cx)
        }

        fn consume(mut self: Pin<&mut Self>, amt: usize) {
            Pin::new(&mut **self).consume(amt)
        }
    }

    impl<'a> AsyncWrite for &'a mut [u8] {
        fn poll_write(mut self: Pin<&mut Self>, _: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
            Poll::Ready(capnp::io::Write::write_all(&mut *self, buf).map(|_| buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    impl<W: ?Sized + AsyncWrite + Unpin> AsyncWrite for &mut W {
        fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
            Pin::new(&mut **self).poll_write(cx, buf)
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
            Pin::new(&mut **self).poll_flush(cx)
        }

        fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
            Pin::new(&mut **self).poll_close(cx)
        }
    }
}

#[cfg(feature="std")]
pub mod async_ext {
    pub use futures::AsyncReadExt;
    pub use futures::AsyncWriteExt;
}

#[cfg(not(feature="std"))]
pub mod async_ext {
    use core::pin::Pin;
    use core::future::Future;
    use core::task::{Context, Poll};
    use alloc::string::ToString;
    use super::{AsyncRead, AsyncWrite};

    pub trait AsyncReadExt: AsyncRead {
        fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> Read<'a, Self>
            where Self: Unpin,
        {
            Read::new(self, buf)
        }

        fn read_exact<'a>(
            &'a mut self,
            buf: &'a mut [u8],
        ) -> ReadExact<'a, Self>
            where Self: Unpin,
        {
            ReadExact::new(self, buf)
        }
    }

    impl<R: AsyncRead + ?Sized> AsyncReadExt for R {}

    pub trait AsyncWriteExt: AsyncWrite {
        fn flush(&mut self) -> Flush<'_, Self>
            where Self: Unpin,
        {
            Flush::new(self)
        }

        fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> WriteAll<'a, Self>
            where Self: Unpin,
        {
            WriteAll::new(self, buf)
        }
    }

    impl<W: AsyncWrite + ?Sized> AsyncWriteExt for W {}

    /// Future for the [`read`](super::AsyncReadExt::read) method.
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Read<'a, R: ?Sized> {
        reader: &'a mut R,
        buf: &'a mut [u8],
    }

    impl<R: ?Sized + Unpin> Unpin for Read<'_, R> {}

    impl<'a, R: AsyncRead + ?Sized + Unpin> Read<'a, R> {
        pub(super) fn new(reader: &'a mut R, buf: &'a mut [u8]) -> Self {
            Read { reader, buf }
        }
    }

    impl<R: AsyncRead + ?Sized + Unpin> Future for Read<'_, R> {
        type Output = capnp::Result<usize>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = &mut *self;
            match Pin::new(&mut this.reader).poll_read(cx, this.buf) {
                Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
                other => other,
            }
        }
    }

    /// Future for the [`read_exact`](super::AsyncReadExt::read_exact) method.
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct ReadExact<'a, R: ?Sized> {
        reader: &'a mut R,
        buf: &'a mut [u8],
    }

    impl<R: ?Sized + Unpin> Unpin for ReadExact<'_, R> {}

    impl<'a, R: AsyncRead + ?Sized + Unpin> ReadExact<'a, R> {
        pub(super) fn new(reader: &'a mut R, buf: &'a mut [u8]) -> Self {
            ReadExact { reader, buf }
        }
    }

    impl<R: AsyncRead + ?Sized + Unpin> Future for ReadExact<'_, R> {
        type Output = capnp::Result<()>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = &mut *self;
            while !this.buf.is_empty() {

                let n = match Pin::new(&mut this.reader).poll_read(cx, this.buf) {
                    Poll::Ready(Ok(n)) => n,
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
                    Poll::Pending => return Poll::Pending,
                };

                // let n = ready!(Pin::new(&mut this.reader).poll_read(cx, this.buf))?;
                {
                    let (_, rest) = core::mem::replace(&mut this.buf, &mut []).split_at_mut(n);
                    this.buf = rest;
                }
                if n == 0 {
                    return Poll::Ready(Err(capnp::Error::failed("unexpected End of File".to_string())))
                }
            }
            Poll::Ready(Ok(()))
        }
    }

    /// Future for the [`flush`](super::AsyncWriteExt::flush) method.
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Flush<'a, W: ?Sized> {
        writer: &'a mut W,
    }

    impl<W: ?Sized + Unpin> Unpin for Flush<'_, W> {}

    impl<'a, W: AsyncWrite + ?Sized + Unpin> Flush<'a, W> {
        pub(super) fn new(writer: &'a mut W) -> Self {
            Flush { writer }
        }
    }

    impl<W> Future for Flush<'_, W>
        where W: AsyncWrite + ?Sized + Unpin,
    {
        type Output = capnp::Result<()>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match Pin::new(&mut *self.writer).poll_flush(cx) {
                Poll::Ready(Err(e)) => Poll::Ready(Err(e.into())),
                other => other,
            }
        }
    }

    /// Future for the [`write_all`](super::AsyncWriteExt::write_all) method.
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct WriteAll<'a, W: ?Sized> {
        writer: &'a mut W,
        buf: &'a [u8],
    }

    impl<W: ?Sized + Unpin> Unpin for WriteAll<'_, W> {}

    impl<'a, W: AsyncWrite + ?Sized + Unpin> WriteAll<'a, W> {
        pub(super) fn new(writer: &'a mut W, buf: &'a [u8]) -> Self {
            WriteAll { writer, buf }
        }
    }

    impl<W: AsyncWrite + ?Sized + Unpin> Future for WriteAll<'_, W> {
        type Output = capnp::Result<()>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = &mut *self;
            while !this.buf.is_empty() {

                let n = match Pin::new(&mut this.writer).poll_write(cx, this.buf) {
                    Poll::Ready(Ok(n)) => n,
                    Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
                    Poll::Pending => return Poll::Pending,
                };

                // let n = ready!(Pin::new(&mut this.writer).poll_write(cx, this.buf))?;
                {
                    let (_, rest) = core::mem::replace(&mut this.buf, &[]).split_at(n);
                    this.buf = rest;
                }
                if n == 0 {
                    return Poll::Ready(Err(capnp::Error::failed("Write Zero".to_string())));
                }
            }

            Poll::Ready(Ok(()))
        }
    }
}