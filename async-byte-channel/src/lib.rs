// Simple in-memory byte stream.

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use futures::{AsyncRead, AsyncWrite};
use std::task::{Poll, Waker};

#[derive(Debug)]
struct Inner {
    buffer: Vec<u8>,
    write_cursor: usize,
    read_cursor: usize,
    write_end_closed: bool,
    read_end_closed: bool,
    read_waker: Option<Waker>,
    write_waker: Option<Waker>,
}

impl Inner {
    fn new() -> Self {
        Self {
            buffer: vec![0; 8096],
            write_cursor: 0,
            read_cursor: 0,
            write_end_closed: false,
            read_end_closed: false,
            read_waker: None,
            write_waker: None,
        }
    }
}

pub struct Sender {
    inner: Arc<Mutex<Inner>>,
}

impl Drop for Sender {
    fn drop(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        inner.write_end_closed = true;
        if let Some(read_waker) = inner.read_waker.take() {
            read_waker.wake();
        }
    }
}

pub struct Receiver {
    inner: Arc<Mutex<Inner>>,
}

impl Drop for Receiver {
    fn drop(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        inner.read_end_closed = true;
        if let Some(write_waker) = inner.write_waker.take() {
            write_waker.wake();
        }
    }
}

pub fn channel() -> (Sender, Receiver) {
    let inner = Arc::new(Mutex::new(Inner::new()));
    let sender = Sender {
        inner: inner.clone(),
    };
    let receiver = Receiver { inner };
    (sender, receiver)
}

impl AsyncRead for Receiver {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut futures::task::Context,
        buf: &mut [u8],
    ) -> futures::task::Poll<Result<usize, futures::io::Error>> {
        let mut inner = self.inner.lock().unwrap();
        if inner.read_cursor == inner.write_cursor {
            if inner.write_end_closed {
                Poll::Ready(Ok(0))
            } else {
                inner.read_waker = Some(cx.waker().clone());
                Poll::Pending
            }
        } else {
            assert!(inner.read_cursor < inner.write_cursor);
            let copy_len = std::cmp::min(buf.len(), inner.write_cursor - inner.read_cursor);
            buf[0..copy_len]
                .copy_from_slice(&inner.buffer[inner.read_cursor..inner.read_cursor + copy_len]);
            inner.read_cursor += copy_len;
            if let Some(write_waker) = inner.write_waker.take() {
                write_waker.wake();
            }
            Poll::Ready(Ok(copy_len))
        }
    }
}

impl AsyncWrite for Sender {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut futures::task::Context,
        buf: &[u8],
    ) -> futures::task::Poll<Result<usize, futures::io::Error>> {
        let mut inner = self.inner.lock().unwrap();
        if inner.read_end_closed {
            return Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "read end closed",
            )));
        }
        if inner.write_cursor == inner.buffer.len() {
            if inner.read_cursor == inner.buffer.len() {
                inner.write_cursor = 0;
                inner.read_cursor = 0;
            } else {
                inner.write_waker = Some(cx.waker().clone());
                return Poll::Pending;
            }
        }

        assert!(inner.write_cursor < inner.buffer.len());

        let copy_len = std::cmp::min(buf.len(), inner.buffer.len() - inner.write_cursor);
        let dest_range = inner.write_cursor..inner.write_cursor + copy_len;
        inner.buffer[dest_range].copy_from_slice(&buf[0..copy_len]);
        inner.write_cursor += copy_len;
        if let Some(read_waker) = inner.read_waker.take() {
            read_waker.wake();
        }
        Poll::Ready(Ok(copy_len))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut futures::task::Context,
    ) -> Poll<Result<(), futures::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: Pin<&mut Self>,
        _cx: &mut futures::task::Context,
    ) -> Poll<Result<(), futures::io::Error>> {
        let mut inner = self.inner.lock().unwrap();
        inner.write_end_closed = true;
        if let Some(read_waker) = inner.read_waker.take() {
            read_waker.wake();
        }
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
pub mod test {
    use futures::task::LocalSpawnExt;
    use futures::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn basic() {
        let (mut sender, mut receiver) = crate::channel();
        let buf: Vec<u8> = vec![1, 2, 3, 4, 5]
            .into_iter()
            .cycle()
            .take(20000)
            .collect();
        let mut pool = futures::executor::LocalPool::new();

        let buf2 = buf.clone();
        pool.spawner()
            .spawn_local(async move {
                sender.write_all(&buf2).await.unwrap();
            })
            .unwrap();

        let mut buf3 = vec![];
        pool.run_until(receiver.read_to_end(&mut buf3)).unwrap();

        assert_eq!(buf.len(), buf3.len());
    }

    #[test]
    fn drop_reader() {
        let (mut sender, receiver) = crate::channel();
        drop(receiver);

        let mut pool = futures::executor::LocalPool::new();
        let result = pool.run_until(sender.write_all(&[0, 1, 2]));
        assert!(result.is_err());
    }
}
