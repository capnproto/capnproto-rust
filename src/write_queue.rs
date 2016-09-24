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
use futures::{self, task, Async, Future, Poll, Complete, Oneshot};

use capnp::{Error};

use serialize::{self, AsOutputSegments};


enum State<W, M> where W: io::Write, M: AsOutputSegments {
    Writing(serialize::Write<W, M>, Complete<M>),
    BetweenWrites(W),
    Empty,
}

/// A write of messages being written.
pub struct WriteQueue<W, M> where W: io::Write, M: AsOutputSegments {
    queue: VecDeque<(M, Complete<M>)>,
    state: State<W, M>,
    task: Option<task::Task>,
}

impl <W, M> WriteQueue<W, M> where W: io::Write, M: AsOutputSegments {
    pub fn new(writer: W) -> WriteQueue<W, M> {
        WriteQueue {
            queue: VecDeque::new(),
            state: State::BetweenWrites(writer),
            task: None,
        }
    }

    /// Enqueues a message to be written.
    pub fn push(&mut self, message: M) -> Oneshot<M> {
        let (complete, oneshot) = futures::oneshot();
        self.queue.push_back((message, complete));

        match self.task.take() {
            Some(t) => t.unpark(),
            None => (),
        }

        oneshot
    }

    /// Returns the number of messages queued to be written, including an in-progress write.
    pub fn len(&mut self) -> usize {
        let in_progress = if let State::Writing(..) = self.state { 1 } else { 0 };
        self.queue.len() + in_progress
    }
}

enum IntermediateState<W, M> where W: io::Write, M: AsOutputSegments {
    WriteDone(M, W),
    StartWrite(M, Complete<M>),
}

impl <W, M> Future for WriteQueue<W, M> where W: io::Write, M: AsOutputSegments {
    type Item = (); // Should never actually terminate.
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next = match self.state {
                State::Writing(ref mut write, ref mut _complete) => {
                    let (w, m) = try_ready!(Future::poll(write));
                    IntermediateState::WriteDone(m, w)
                }
                State::BetweenWrites(ref mut _writer) => {
                    match self.queue.pop_front() {
                        Some((m, complete)) => {
                            IntermediateState::StartWrite(m, complete)
                        }
                        None => {
                            // if queue is empty, park task.
                            self.task = Some(task::park());
                            return Ok(Async::NotReady)
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
            }
        }
    }
}
