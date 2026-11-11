// Copyright (c) 2016 Sandstorm Development Group, Inc. and contributors
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

use std::future::Future;

use futures_channel::oneshot;
use futures_util::{AsyncWrite, AsyncWriteExt, FutureExt, StreamExt, TryFutureExt};

use capnp::Error;

use crate::serialize::AsOutputSegments;

enum Item<M>
where
    M: AsOutputSegments,
{
    Message(M, oneshot::Sender<M>),
    Done(Result<(), Error>, oneshot::Sender<()>),
}

/// A handle that allows messages to be sent to a write queue.
pub struct Sender<M>
where
    M: AsOutputSegments,
{
    sender: futures_channel::mpsc::UnboundedSender<Item<M>>,
    in_flight: std::sync::Arc<std::sync::atomic::AtomicI32>,
}

impl<M> Clone for Sender<M>
where
    M: AsOutputSegments,
{
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            in_flight: self.in_flight.clone(),
        }
    }
}

/// Creates a new write queue that wraps the given `AsyncWrite`.
///
/// Returns `(sender, task)`, where `sender` can be used to push writes onto the
/// queue, and `task` is a future that performs the work of the writes. The queue
/// will run as long as `task` is polled, until either `sender.terminate()` is
/// called or `sender` and all of its clones are dropped.
///
/// Messages that are already queued when the task starts writing are written
/// back to back and share a single `flush()`. Messages sent while that batch is
/// being written go into the next batch.
pub fn write_queue<W, M>(mut writer: W) -> (Sender<M>, impl Future<Output = Result<(), Error>>)
where
    W: AsyncWrite + Unpin,
    M: AsOutputSegments,
{
    let (tx, mut rx) = futures_channel::mpsc::unbounded::<Item<M>>();

    let in_flight = std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0));

    let sender = Sender {
        sender: tx,
        in_flight: in_flight.clone(),
    };

    let queue = async move {
        let mut written = Vec::new();
        while let Some(mut item) = rx.next().await {
            let mut batch_remaining = in_flight.load(std::sync::atomic::Ordering::SeqCst);
            loop {
                match item {
                    Item::Message(m, returner) => {
                        let result = crate::serialize::write_message(&mut writer, &m).await;
                        in_flight.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                        result?;
                        written.push((m, returner));
                        batch_remaining -= 1;
                    }
                    Item::Done(r, finisher) => {
                        flush_written(&mut writer, &mut written).await?;
                        let _ = finisher.send(());
                        return r;
                    }
                }
                if batch_remaining <= 0 {
                    break;
                }
                match rx.next().now_or_never() {
                    Some(Some(next)) => item = next,
                    _ => break,
                }
            }
            flush_written(&mut writer, &mut written).await?;
        }
        Ok(())
    };

    (sender, queue)
}

/// Flushes `writer`, then hands each written message back to its sender.
async fn flush_written<W, M>(
    writer: &mut W,
    written: &mut Vec<(M, oneshot::Sender<M>)>,
) -> Result<(), Error>
where
    W: AsyncWrite + Unpin,
{
    if written.is_empty() {
        return Ok(());
    }
    writer.flush().await?;
    for (m, returner) in written.drain(..) {
        let _ = returner.send(m);
    }
    Ok(())
}

fn _assert_kinds() {
    fn _assert_send<T: Send>(_x: T) {}
    fn _assert_sync<T: Sync>() {}
    fn _assert_write_queue_send<W: AsyncWrite + Unpin + Send, M: AsOutputSegments + Sync + Send>(
        w: W,
    ) {
        let (s, f) = write_queue::<W, M>(w);
        _assert_send(s);
        _assert_send(f);
    }
    fn _assert_write_queue_send_2<W: AsyncWrite + Unpin + Send>(w: W) {
        let (s, f) = write_queue::<W, capnp::message::Builder<capnp::message::HeapAllocator>>(w);
        _assert_send(s);
        _assert_send(f);
    }
}

impl<M> Sender<M>
where
    M: AsOutputSegments,
{
    /// Enqueues a message to be written. Returns the message once the write
    /// has completed. Dropping the returned future does *not* cancel the write.
    pub fn send(&mut self, message: M) -> impl Future<Output = Result<M, Error>> + Unpin {
        self.in_flight
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let (complete, oneshot) = oneshot::channel();

        let _ = self.sender.unbounded_send(Item::Message(message, complete));

        oneshot.map_err(|oneshot::Canceled| Error::disconnected("WriteQueue has terminated".into()))
    }

    /// Returns the number of messages queued to be written.
    pub fn len(&self) -> usize {
        let result = self.in_flight.load(std::sync::atomic::Ordering::SeqCst);
        assert!(result >= 0);
        result as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Commands the queue to stop writing messages once it is empty. After this method has been called,
    /// any new calls to `send()` will return a future that immediately resolves to an error.
    /// If the passed-in `result` is an error, then the `WriteQueue` will resolve to that error.
    pub fn terminate(
        &mut self,
        result: Result<(), Error>,
    ) -> impl Future<Output = Result<(), Error>> + Unpin {
        let (complete, receiver) = oneshot::channel();

        let _ = self.sender.unbounded_send(Item::Done(result, complete));

        receiver
            .map_err(|oneshot::Canceled| Error::disconnected("WriteQueue has terminated".into()))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::future::Future;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::task::{Context, Poll};

    use capnp::message::{Builder, HeapAllocator, ReaderOptions};
    use futures::executor::block_on;
    use futures_util::{AsyncWrite, FutureExt};

    use super::{write_queue, Sender};

    type Message = Builder<HeapAllocator>;

    #[derive(Default)]
    struct RecordingWriter {
        bytes: Vec<u8>,
        flushes: usize,
        pending_flushes: usize,
        on_first_write: Option<Box<dyn FnOnce()>>,
    }

    impl AsyncWrite for RecordingWriter {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            if let Some(f) = self.on_first_write.take() {
                f();
            }
            self.bytes.extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            if self.pending_flushes > 0 {
                self.pending_flushes -= 1;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            self.flushes += 1;
            Poll::Ready(Ok(()))
        }

        fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    fn message(value: u64) -> Message {
        let mut message = Builder::new_default();
        message
            .init_root::<capnp::any_pointer::Builder>()
            .initn_as::<capnp::primitive_list::Builder<u64>>(1)
            .set(0, value);
        message
    }

    fn values(mut bytes: &[u8]) -> Vec<u64> {
        let mut values = Vec::new();
        while !bytes.is_empty() {
            let reader =
                capnp::serialize::read_message_from_flat_slice(&mut bytes, ReaderOptions::new())
                    .unwrap();
            let list: capnp::primitive_list::Reader<u64> = reader.get_root().unwrap();
            values.push(list.get(0));
        }
        values
    }

    #[test]
    fn queued_messages_share_one_flush() {
        let mut writer = RecordingWriter::default();
        let (mut sender, queue) = write_queue::<_, Message>(&mut writer);
        let sent: Vec<_> = (0..3).map(|i| sender.send(message(i))).collect();
        let terminated = sender.terminate(Ok(()));

        block_on(queue).unwrap();
        for s in sent {
            block_on(s).unwrap();
        }
        block_on(terminated).unwrap();

        assert_eq!(writer.flushes, 1);
        assert_eq!(values(&writer.bytes), [0, 1, 2]);
    }

    #[test]
    fn terminate_flushes_messages_written_before_it() {
        let mut writer = RecordingWriter::default();
        let (mut sender, queue) = write_queue::<_, Message>(&mut writer);
        let first = sender.send(message(0));
        let terminated = sender.terminate(Ok(()));
        let after_terminate = sender.send(message(1));

        block_on(queue).unwrap();

        let returned = block_on(first).unwrap();
        let list: capnp::primitive_list::Reader<u64> = returned.get_root_as_reader().unwrap();
        assert_eq!(list.get(0), 0);
        block_on(terminated).unwrap();
        assert!(block_on(after_terminate).is_err());
        assert_eq!(writer.flushes, 1);
        assert_eq!(values(&writer.bytes), [0]);
    }

    #[test]
    fn messages_sent_during_a_write_get_their_own_flush() {
        let slot: Rc<RefCell<Option<Sender<Message>>>> = Rc::new(RefCell::new(None));
        let mut writer = RecordingWriter::default();
        let late_sender = slot.clone();
        writer.on_first_write = Some(Box::new(move || {
            let mut sender = late_sender.borrow_mut().take().unwrap();
            drop(sender.send(message(1)));
        }));

        let (mut sender, queue) = write_queue::<_, Message>(&mut writer);
        *slot.borrow_mut() = Some(sender.clone());
        let first = sender.send(message(0));
        drop(sender);

        block_on(queue).unwrap();
        block_on(first).unwrap();

        assert_eq!(writer.flushes, 2);
        assert_eq!(values(&writer.bytes), [0, 1]);
    }

    #[test]
    fn send_resolves_after_flush() {
        let mut writer = RecordingWriter {
            pending_flushes: 1,
            ..Default::default()
        };
        let (mut sender, queue) = write_queue::<_, Message>(&mut writer);
        let mut sent = sender.send(message(0));
        drop(sender);

        let mut queue = Box::pin(queue);
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(queue.as_mut().poll(&mut cx).is_pending());
        assert!((&mut sent).now_or_never().is_none());

        assert!(queue.as_mut().poll(&mut cx).is_ready());
        assert!(sent.now_or_never().unwrap().is_ok());
        drop(queue);
        assert_eq!(writer.flushes, 1);
    }
}
