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

use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use futures::future::Future;
use futures::channel::oneshot;
use futures::{AsyncWrite, TryFutureExt};

use capnp::{Error};

use crate::serialize::{self, AsOutputSegments};

enum State<W, M> where W: AsyncWrite + Unpin, M: AsOutputSegments {
    Writing(serialize::Write<W, M>, oneshot::Sender<M>),
    BetweenWrites(W),
    Empty,
}

/// A queue of messages being written.
#[must_use = "futures do nothing unless polled"]
pub struct WriteQueue<W, M> where W: AsyncWrite + Unpin, M: AsOutputSegments {
    inner: Rc<RefCell<Inner<M>>>,
    state: State<W, M>,
}

impl <W, M> Unpin for WriteQueue<W, M> where W: AsyncWrite + Unpin, M: AsOutputSegments {}

struct Inner<M> {
    queue: VecDeque<(M, oneshot::Sender<M>)>,
    sender_count: usize,
    task: Option<Waker>,

    // If set, then the queue has been requested to end, and we should complete the oneshot once
    // the queue has been emptied.
    end_notifier: Option<(Result<(), Error>, oneshot::Sender<()>)>
}

/// A handle that allows message to be sent to a `WriteQueue`.
pub struct Sender<M> where M: AsOutputSegments {
    inner: Weak<RefCell<Inner<M>>>,
}

impl <M> Clone for Sender<M> where M: AsOutputSegments {
    fn clone(&self) -> Sender<M> {
        match self.inner.upgrade() {
            None => (),
            Some(inner) => {
                inner.borrow_mut().sender_count += 1;
            }
        }
        Sender { inner: self.inner.clone() }
    }
}

impl <M> Drop for Sender<M> where M: AsOutputSegments {
    fn drop(&mut self) {
        match self.inner.upgrade() {
            None => (),
            Some(inner) => {
                inner.borrow_mut().sender_count -= 1;
            }
        }
    }
}

/// Creates a new WriteQueue that wraps the given writer.
pub fn write_queue<W, M>(writer: W) -> (Sender<M>, WriteQueue<W, M>)
    where W: AsyncWrite + Unpin, M: AsOutputSegments
{
    let inner = Rc::new(RefCell::new(Inner {
        queue: VecDeque::new(),
        task: None,
        sender_count: 1,
        end_notifier: None,
    }));

    let sender = Sender { inner: Rc::downgrade(&inner) };

    let queue = WriteQueue {
        inner: inner,
        state: State::BetweenWrites(writer),
    };

    (sender, queue)
}

impl <M> Sender<M> where M: AsOutputSegments + 'static {
    /// Enqueues a message to be written.
    pub fn send(&mut self, message: M) -> Box<dyn Future<Output=Result<M,Error>> + Unpin> {
        let (complete, oneshot) = oneshot::channel();

        match self.inner.upgrade() {
            None => (),
            Some(rc_inner) => {
                if rc_inner.borrow().end_notifier.is_some() {
                    drop(complete)
                } else {
                    rc_inner.borrow_mut().queue.push_back((message, complete));
                }

                match rc_inner.borrow_mut().task.take() {
                    Some(t) => t.wake(),
                    None => (),
                }
            }
        }

        Box::new(
            oneshot.map_err(
                |oneshot::Canceled| Error::disconnected("WriteQueue has terminated".into())))
    }

    /// Returns the number of messages queued to be written, not including any in-progress write.
    pub fn len(&mut self) -> usize {
        match self.inner.upgrade() {
            None => 0,
            Some(rc_inner) => rc_inner.borrow().queue.len(),
        }

    }

    /// Commands the queue to stop writing messages once it is empty. After this method has been called,
    /// any new calls to `send()` will return a future that immediately resolves to an error.
    /// If the passed-in `result` is an error, then the `WriteQueue` will resolve to that error.
    pub fn terminate(&mut self, result: Result<(), Error>) -> Box<dyn Future<Output=Result<(),Error>> + Unpin> {
        let (complete, receiver) = oneshot::channel();

        match self.inner.upgrade() {
            None => (),
            Some(rc_inner) => {
                // TODO: what if end_notifier is already full? Maybe it should be a vector?
                rc_inner.borrow_mut().end_notifier = Some((result, complete));

                match rc_inner.borrow_mut().task.take() {
                    Some(t) => t.wake(),
                    None => (),
                }
            }
        }

        Box::new(
            receiver.map_err(
                |oneshot::Canceled| Error::disconnected("WriteQueue has terminated".into())))
    }
}

enum IntermediateState<W, M> where W: AsyncWrite + Unpin, M: AsOutputSegments {
    WriteDone(M, W),
    StartWrite(M, oneshot::Sender<M>),
    Resolve,
}

impl <W, M> Future for WriteQueue<W, M> where W: AsyncWrite + Unpin, M: AsOutputSegments + Unpin {
    type Output = Result<W, Error>; // Resolves successfully when all senders have been dropped and all messages written.

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            let next = match self.state {
                State::Writing(ref mut write, ref mut _complete) => {
                    let (w, m) = match Future::poll(Pin::new(write), cx) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                        Poll::Ready(Ok(x)) => x,
                    };
                    IntermediateState::WriteDone(m, w)
                }
                State::BetweenWrites(ref mut _writer) => {
                    let front = self.inner.borrow_mut().queue.pop_front();
                    match front {
                        Some((m, complete)) => {
                            IntermediateState::StartWrite(m, complete)
                        }
                        None => {
                            let count = self.inner.borrow().sender_count;
                            let ended = self.inner.borrow().end_notifier.is_some();
                            if count == 0 || ended {
                                IntermediateState::Resolve
                            } else {
                                self.inner.borrow_mut().task = Some(cx.waker().clone());
                                return Poll::Pending;
                            }
                        }
                    }
                }
                State::Empty => unreachable!(),
            };

            match next {
                IntermediateState::WriteDone(m, w) => {
                    match ::std::mem::replace(&mut self.state, State::BetweenWrites(w)) {
                        State::Writing(_, complete) => {
                            complete.send(m).unwrap_or(());
                        }
                        _ => unreachable!(),
                    }
                }
                IntermediateState::StartWrite(m, c) => {
                    let new_state = match ::std::mem::replace(&mut self.state, State::Empty) {
                        State::BetweenWrites(w) => {
                            State::Writing(serialize::write_message(w, m), c)
                        }
                        _ => unreachable!(),
                    };
                    self.state = new_state;
                }
                IntermediateState::Resolve => {
                    let end_notifier = self.inner.borrow_mut().end_notifier.take();
                    match end_notifier {
                        None => (),
                        Some((result, complete)) => {
                            complete.send(()).unwrap_or(());
                            if let Err(e) = result {
                                return Poll::Ready(Err(e))
                            }
                        }
                    }
                    match ::std::mem::replace(&mut self.state, State::Empty) {
                        State::BetweenWrites(w) => {
                            return Poll::Ready(Ok(w));
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }
}
