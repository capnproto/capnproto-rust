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
use futures_util::{AsyncWrite, AsyncWriteExt, StreamExt, TryFutureExt};

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
        while let Some(item) = rx.next().await {
            match item {
                Item::Message(m, returner) => {
                    let result = crate::serialize::write_message(&mut writer, &m).await;
                    in_flight.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                    result?;
                    writer.flush().await?;
                    let _ = returner.send(m);
                }
                Item::Done(r, finisher) => {
                    let _ = finisher.send(());
                    return r;
                }
            }
        }
        Ok(())
    };

    (sender, queue)
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
