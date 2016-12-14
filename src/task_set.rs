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

use std::cell::{RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

use stack::Stack;

enum Slot<T> {
    Next(usize),
    Data(T),
}

struct Inner<T, E> {
   // A slab of futures that are being executed. Each slot in this vector is
    // either an active future or a pointer to the next empty slot. This is used
    // to get O(1) deallocation in the slab and O(1) allocation.
    //
    // The `next_future` field is the next slot in the `futures` array that's a
    // `Slot::Next` variant. If it points to the end of the array then the array
    // is full.
    futures: Vec<Slot<Box<Future<Item=T, Error=E>>>>,
    next_future: usize,

    stack: Arc<Stack<usize>>,
    reaper: Box<TaskReaper<T, E>>,

    terminate_with: Option<Result<(), E>>,

    handle_count: usize,
    task: Option<::futures::task::Task>,
}


#[must_use = "a TaskSet does nothing unless polled"]
pub struct TaskSet<T, E> {
    inner: Rc<RefCell<Inner<T, E>>>,
}

impl<T, E> TaskSet<T, E> where T: 'static, E: 'static {
    pub fn new(reaper: Box<TaskReaper<T, E>>)
               -> (TaskSetHandle<T, E>, TaskSet<T, E>)
        where E: 'static, T: 'static, E: ::std::fmt::Debug,
    {
        let inner = Rc::new(RefCell::new(Inner {
            futures: Vec::new(),
            next_future: 0,
            stack: Arc::new(Stack::new()),
            reaper: reaper,
            terminate_with: None,
            handle_count: 1,
            task: None,
        }));

        let weak_inner = Rc::downgrade(&inner);

        let set = TaskSet {
            inner: inner,
        };

        let handle = TaskSetHandle {
            inner: weak_inner,
        };

        (handle, set)
    }
}

pub struct TaskSetHandle<T, E> {
    inner: Weak<RefCell<Inner<T, E>>>,
}

impl<T, E> Clone for TaskSetHandle<T, E> {
    fn clone(&self) -> TaskSetHandle<T, E> {
        match self.inner.upgrade() {
            None => (),
            Some(inner) => {
                inner.borrow_mut().handle_count += 1;
            }
        }
        TaskSetHandle {
            inner: self.inner.clone()
        }
    }
}

impl <T, E> Drop for TaskSetHandle<T, E> {
    fn drop(&mut self) {
        match self.inner.upgrade() {
            None => (),
            Some(inner) => {
                inner.borrow_mut().handle_count -= 1;
            }
        }
    }
}

impl <T, E> TaskSetHandle<T, E> where T: 'static, E: 'static {
    pub fn add<F>(&mut self, promise: F)
        where F: Future<Item=T, Error=E> + 'static
    {
        match self.inner.upgrade() {
            None => (),
            Some(rc_inner) => {
                let ref mut inner = *rc_inner.borrow_mut();
                let future = Box::new(promise);

                let added_idx = inner.next_future;
                if inner.next_future == inner.futures.len() {
                    inner.futures.push(Slot::Data(future));
                    inner.next_future += 1;
                } else {
                    match ::std::mem::replace(&mut inner.futures[inner.next_future],
                                              Slot::Data(future)) {
                        Slot::Next(next) => inner.next_future = next,
                        Slot::Data(_) => unreachable!(),
                    }
                }

                inner.stack.push(added_idx);

                match inner.task.take() {
                    Some(t) => t.unpark(),
                    None => (),
                }
            }
        }
    }

    pub fn terminate(&mut self, result: Result<(), E>) {
        match self.inner.upgrade() {
            None => (),
            Some(rc_inner) => {
                let ref mut inner = *rc_inner.borrow_mut();
                inner.terminate_with = Some(result);

                match inner.task.take() {
                    Some(t) => t.unpark(),
                    None => (),
                }
            }
        }
    }
}

pub trait TaskReaper<T, E> where T: 'static, E: 'static
{
    fn task_succeeded(&mut self, _value: T) {}
    fn task_failed(&mut self, error: E);
}

impl <T, E> Future for TaskSet<T, E> where T: 'static, E: 'static {
    type Item = ();
    type Error = E;

    fn poll(&mut self) -> ::futures::Poll<Self::Item, Self::Error> {
        let ref mut inner = *self.inner.borrow_mut();

        match inner.terminate_with.take() {
            None => (),
            Some(Ok(v)) => return Ok(::futures::Async::Ready(v)),
            Some(Err(e)) => return Err(e),
        }

        for idx in inner.stack.drain() {
            match inner.futures[idx] {
                Slot::Next(_) => unreachable!(),
                Slot::Data(ref mut f) => {
                    let event = ::futures::task::UnparkEvent::new(inner.stack.clone(), idx);
                    match ::futures::task::with_unpark_event(event, || f.poll()) {
                        Ok(::futures::Async::NotReady) => continue,
                        Ok(::futures::Async::Ready(v)) => {
                            inner.reaper.task_succeeded(v);
                        }
                        Err(e) => {
                            inner.reaper.task_failed(e);
                        }
                    }
                }
            }
            inner.futures[idx] = Slot::Next(inner.next_future);
            inner.next_future = idx;
        }

        if inner.futures.len() == 0 && inner.handle_count == 0 {
            Ok(::futures::Async::Ready(()))
        } else {
            inner.task = Some(::futures::task::park());
            Ok(::futures::Async::NotReady)
        }
    }
}
