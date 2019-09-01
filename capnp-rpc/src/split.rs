// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
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
use futures::{Future};

use std::cell::RefCell;
use std::rc::{Rc};

enum State<T1, T2, E>
    where E: Clone,
{
    NotReady(Option<Waker>, Option<Waker>),
    Ready(Option<Result<T1, E>>, Option<Result<T2, E>>),
}

struct Inner<F, T1, T2, E>
    where F: Future<Output=Result<(T1, T2),E>> + Unpin,
          E: Clone,
{
    original_future: RefCell<F>,
    state: RefCell<State<T1, T2, E>>
}

pub struct SplitLeft<F, T1, T2, E>
    where F: Future<Output=Result<(T1, T2),E>> + Unpin,
          E: Clone,
{
    inner: Rc<Inner<F, T1, T2, E>>,
}

pub struct SplitRight<F, T1, T2, E>
    where F: Future<Output=Result<(T1, T2),E>> + Unpin,
          E: Clone,
{
    inner: Rc<Inner<F, T1, T2, E>>,
}

pub fn split<F, T1, T2, E>(f: F) -> (SplitLeft<F, T1, T2, E>, SplitRight<F, T1, T2, E>)
    where F: Future<Output=Result<(T1, T2), E>> + Unpin,
          E: Clone,
{
    let inner =
        Rc::new(Inner {
            original_future: RefCell::new(f),
            state: RefCell::new(State::NotReady(None, None)),
        });
    (SplitLeft { inner: inner.clone() }, SplitRight { inner: inner })
}

impl <F, T1, T2, E> Drop for SplitLeft<F, T1, T2, E>
    where F: Future<Output=Result<(T1, T2),E>> + Unpin,
          E: Clone,
{
    fn drop(&mut self) {
        match *self.inner.state.borrow_mut() {
            State::NotReady(_, ref mut right_task) => {
                if let Some(t) = right_task.take() {
                    t.wake()
                }
            }
            _ => ()
        }
    }
}

impl <F, T1, T2, E> Drop for SplitRight<F, T1, T2, E>
    where F: Future<Output=Result<(T1, T2),E>> + Unpin,
          E: Clone,
{
    fn drop(&mut self) {
        match *self.inner.state.borrow_mut() {
            State::NotReady(ref mut left_task, _) => {
                if let Some(t) = left_task.take() {
                    t.wake()
                }
            }
            _ => ()
        }
    }
}

impl <F, T1, T2, E> Future for SplitLeft<F, T1, T2, E>
    where F: Future<Output=Result<(T1, T2),E>> + Unpin,
          E: Clone,
{
    type Output = Result<T1, E>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match *self.inner.state.borrow_mut() {
            State::NotReady(_, _) => (),
            State::Ready(ref mut t1, _) => {
                match t1.take() {
                    Some(r) => return Poll::Ready(r),
                    None => panic!("polled already-done future"),
                }

            }
        }

        let polled = Pin::new(&mut *self.inner.original_future.borrow_mut()).poll(cx);
        let done_val = match polled {
            Poll::Ready(v) => v,
            Poll::Pending => {
                match *self.inner.state.borrow_mut() {
                    State::NotReady(ref mut left_task, _) => {
                        *left_task = Some(cx.waker().clone());
                    }
                    _ => unreachable!()
                }
                return Poll::Pending;
            }
        };

        match *self.inner.state.borrow_mut() {
            State::NotReady(_, ref mut right_task) => {
                if let Some(t) = right_task.take() {
                    t.wake()
                }
            }
            _ => unreachable!()
        }

        match done_val {
            Ok((t1, t2)) => {
                *self.inner.state.borrow_mut() = State::Ready(None, Some(Ok(t2)));
                Poll::Ready(Ok(t1))
            }
            Err(e) => {
                *self.inner.state.borrow_mut() = State::Ready(None, Some(Err(e.clone())));
                Poll::Ready(Err(e))
            }
        }
    }
}

impl <F, T1, T2, E> Future for SplitRight<F, T1, T2, E>
    where F: Future<Output=Result<(T1, T2),E>> + Unpin,
          E: Clone,
{
    type Output = Result<T2, E>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match *self.inner.state.borrow_mut() {
            State::NotReady(_, _) => (),
            State::Ready(_, ref mut t2) => {
                match t2.take() {
                    Some(r) => return Poll::Ready(r),
                    None => panic!("polled already-done future"),
                }
            }
        }

        let polled = Pin::new(&mut *self.inner.original_future.borrow_mut()).poll(cx);
        let done_val = match polled {
            Poll::Ready(r) => r,
            Poll::Pending => {
                match *self.inner.state.borrow_mut() {
                    State::NotReady(_, ref mut right_task) => {
                        *right_task = Some(cx.waker().clone());
                    }
                    _ => unreachable!()
                }
                return Poll::Pending;
            }
        };

        match *self.inner.state.borrow_mut() {
            State::NotReady(ref mut left_task, _) => {
                if let Some(t) = left_task.take() {
                    t.wake()
                }
            }
            _ => unreachable!()
        }

        match done_val {
            Ok((t1, t2)) => {
                *self.inner.state.borrow_mut() = State::Ready(Some(Ok(t1)), None);
                Poll::Ready(Ok(t2))
            }
            Err(e) => {
                *self.inner.state.borrow_mut() = State::Ready(Some(Err(e.clone())), None);
                Poll::Ready(Err(e))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use futures::{Future};
    use std::cell::RefCell;
    use std::rc::{Rc};
    use super::split;

    #[test]
    fn drop_in_poll() {
        let slot = Rc::new(RefCell::new(None));
        let slot2 = slot.clone();
        let (f1, f2) = split(::futures::future::lazy(move |_| {
            drop(slot2.borrow_mut().take().unwrap());
            Ok::<_,()>((11,"foo"))
        }));
        let future2 = Box::new(f2) as Box<dyn Future<Output=_>>;
        *slot.borrow_mut() = Some(future2);

        let mut exec = futures::executor::LocalPool::new();
        assert_eq!(exec.run_until(f1).unwrap(), 11);
    }
}
