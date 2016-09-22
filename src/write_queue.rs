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
use futures::{self, stream, task, Future, Poll, Complete, Oneshot};

use capnp::{Error};

use serialize::{self, AsOutputSegments};


enum State<W, M> where W: io::Write, M: AsOutputSegments {
    Writing(serialize::Write<W, M>, Complete<M>),
    Empty(W),
}

pub struct WriteQueue<W, M> where W: io::Write, M: AsOutputSegments {
    queue: VecDeque<(M, Complete<Result<M, Error>>)>,
    state: State<W, M>,
    task: Option<task::Task>,
}

impl <W, M> WriteQueue<W, M> where W: io::Write, M: AsOutputSegments {
    pub fn new(writer: W) -> WriteQueue<W, M> {
        WriteQueue {
            queue: VecDeque::new(),
            state: State::Empty(writer),
            task: None,
        }
    }

    pub fn push(&mut self, message: M) -> Oneshot<Result<M,Error>> {
        let (complete, oneshot) = futures::oneshot();

        // If Empty, then transition to Writing....
        self.queue.push_back((message, complete));

        oneshot
    }
}

impl <W, M> stream::Stream for WriteQueue<W, M> where W: io::Write, M: AsOutputSegments {
    type Item = M;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.state {
            State::Writing(ref mut write, ref mut _complete) => {
                let (_w, _m) = try_ready!(Future::poll(write));
                // complete.complete(m) ...
                ()
            }
            State::Empty(ref mut _writer) => {
                // if queue is empty, park task.
                // otherwise, grab something off the queue and start writing it.
                unimplemented!()
            }
        }

        unimplemented!()
    }
}
