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

use std::io;
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use futures::{self, task, Async, Future, Poll};
use futures::sync::oneshot;

use capnp::{Error};

use serialize::{self, AsOutputSegments};


enum State<W, M> where W: io::Write, M: AsOutputSegments {
    Writing(serialize::Write<W, M>, oneshot::Sender<M>),
    BetweenWrites(W),
    Empty,
}

/// A queue of messages being written.
pub struct WriteQueue<W, M> where W: io::Write, M: AsOutputSegments {
    inner: Rc<RefCell<Inner<M>>>,
    state: State<W, M>,
}

struct Inner<M> {
    queue: VecDeque<(M, oneshot::Sender<M>)>,
    sender_count: usize,
    task: Option<task::Task>,

    // If set, then the queue has been requested to end, and we should complete the oneshot once
    // the queue has been emptied.
    end_notifier: Option<oneshot::Sender<()>>
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

pub fn write_queue<W, M>(writer: W) -> (Sender<M>, WriteQueue<W, M>)
    where W: io::Write, M: AsOutputSegments
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

impl <M> Sender<M> where M: AsOutputSegments {
    /// Enqueues a message to be written.
    pub fn send(&mut self, message: M) -> oneshot::Receiver<M> {
        let (complete, oneshot) = futures::oneshot();

        match self.inner.upgrade() {
            None => (),
            Some(rc_inner) => {
                if rc_inner.borrow().end_notifier.is_some() {
                    drop(complete)
                } else {
                    rc_inner.borrow_mut().queue.push_back((message, complete));
                }

                match rc_inner.borrow_mut().task.take() {
                    Some(t) => t.unpark(),
                    None => (),
                }
            }
        }

        oneshot
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
    pub fn end(&mut self) -> oneshot::Receiver<()> {
        let (complete, receiver) = futures::oneshot();

        match self.inner.upgrade() {
            None => (),
            Some(rc_inner) => {
                // TODO: what if end_notifier is already full? Maybe it should be a vector?
                rc_inner.borrow_mut().end_notifier = Some(complete);

                match rc_inner.borrow_mut().task.take() {
                    Some(t) => t.unpark(),
                    None => (),
                }
            }
        }

        receiver
    }
}

enum IntermediateState<W, M> where W: io::Write, M: AsOutputSegments {
    WriteDone(M, W),
    StartWrite(M, oneshot::Sender<M>),
    Resolve,
}

impl <W, M> Future for WriteQueue<W, M> where W: io::Write, M: AsOutputSegments {
    type Item = W; // Resolves when all senders have been dropped and all messages written.
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next = match self.state {
                State::Writing(ref mut write, ref mut _complete) => {
                    let (w, m) = try_ready!(Future::poll(write));
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
                                self.inner.borrow_mut().task = Some(task::park());
                                return Ok(Async::NotReady)
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
                            complete.complete(m)
                        }
                        _ => unreachable!(),
                    }
                }
                IntermediateState::StartWrite(m, c) => {
                    let new_state = match ::std::mem::replace(&mut self.state, State::Empty) {
                        State::BetweenWrites(w) => {
                            State::Writing(::serialize::write_message(w, m), c)
                        }
                        _ => unreachable!(),
                    };
                    self.state = new_state;
                }
                IntermediateState::Resolve => {
                    let ender = self.inner.borrow_mut().end_notifier.take();
                    match ender {
                        None => (),
                        Some(complete) => complete.complete(()),
                    }
                    match ::std::mem::replace(&mut self.state, State::Empty) {
                        State::BetweenWrites(w) => {
                            return Ok(Async::Ready(w))
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }
}
