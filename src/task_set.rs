// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors
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

use futures::{Future, Stream};
use futures::sync::oneshot;

use std::cell::{RefCell};
use std::rc::{Rc};
use std::sync::Arc;

use stack::Stack;

struct Slab<T> {
    slots: Vec<Slot<T>>,
    next_free: usize,
}

enum Slot<T> {
    Next(usize),
    Data(T),
}

impl <T> Slab<T> {
    fn new() -> Slab<T> {
        Slab {
            slots: Vec::new(),
            next_free: 0,
        }
    }

    fn push(value: T) -> usize {
        unimplemented!()
    }
}

// we need a set that impls EventSet. Let's just wrap ::std::collections::HashSet<>;

#[must_use = "a TaskSet does nothing unless polled"]
pub struct TaskSet<T, E> {

    // A slab of futures that are being executed. Each slot in this vector is
    // either an active future or a pointer to the next empty slot. This is used
    // to get O(1) deallocation in the slab and O(1) allocation.
    //
    // The `next_future` field is the next slot in the `futures` array that's a
    // `Slot::Next` variant. If it points to the end of the array then the array
    // is full.
    futures: Vec<Slot<Box<Future<Item=(), Error=()>>>>,
    next_future: usize,

    stack: Arc<Stack<usize>>,
    reaper: Rc<RefCell<Box<TaskReaper<T, E>>>>,
}

impl<T, E> TaskSet<T, E> where T: 'static, E: 'static {
    pub fn new(reaper: Box<TaskReaper<T, E>>)
               -> TaskSet<T, E>
        where E: 'static, T: 'static, E: ::std::fmt::Debug,
    {
        TaskSet {
            futures: Vec::new(),
            next_future: 0,
            stack: Arc::new(Stack::new()),
            reaper: Rc::new(RefCell::new(reaper)),
        }
    }

    pub fn add<F>(&mut self, promise: F)
        where F: Future<Item=T, Error=E> + 'static
    {
        let reaper = self.reaper.clone();
        let future = Box::new(promise.then(move |r| {
            match r {
                Ok(v) => reaper.borrow_mut().task_succeeded(v),
                Err(e) => reaper.borrow_mut().task_failed(e),
            }
            Ok(())
        }));

        if self.next_future == self.futures.len() {
            self.futures.push(Slot::Data(future));
            self.next_future += 1;
        } else {
            match ::std::mem::replace(&mut self.futures[self.next_future],
                                      Slot::Data(future)) {
                Slot::Next(next) => self.next_future = next,
                Slot::Data(_) => unreachable!(),
            }
        }

        // maybe add the new thing to the event set?
        unimplemented!()
    }
}

pub trait TaskReaper<T, E> where T: 'static, E: 'static
{
    fn task_succeeded(&mut self, _value: T) {}
    fn task_failed(&mut self, error: E);
}

impl <T, E> Future for TaskSet<T, E> {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> ::futures::Poll<Self::Item, Self::Error> {
        for idx in self.stack.drain() {
            match self.futures[idx] {
                Slot::Next(_) => unreachable!(),
                Slot::Data(ref mut f) => {
                    // unpark event...
                }
            }
        }
        unimplemented!()
    }
}
