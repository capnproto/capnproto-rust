// Copyright (c) 2013-2017 Sandstorm Development Group, Inc. and contributors
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

use futures::{task, Future};

use std::cell::{Cell, RefCell};
use std::rc::{Rc};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;

struct Unparker {
    original_needs_poll: AtomicBool,

    // It's sad that we need a mutex here, even though we know that the RPC
    // system is restricted to a single thread.
    tasks: Mutex<HashMap<u64, task::Task>>,
}

impl Unparker {
    fn new(original_needs_poll: AtomicBool) -> Unparker {
        Unparker {
            original_needs_poll: original_needs_poll,
            tasks: Mutex::new(HashMap::new()),
        }
    }

    fn insert(&self, idx: u64, task: task::Task) {
        self.tasks.lock().unwrap().insert(idx, task);
    }

    fn remove(&self, idx: u64) {
        self.tasks.lock().unwrap().remove(&idx);
    }

    fn unpark(&self) {
        self.original_needs_poll.store(true, Ordering::SeqCst);
        let tasks = ::std::mem::replace(&mut *self.tasks.lock().unwrap(), HashMap::new());
        for (_, task) in tasks {
            task.unpark();
        }
    }
}

impl task::EventSet for Unparker {
    fn insert(&self, _id: usize) {
        self.unpark();
    }
}

struct ForkedPromiseInner<F> where F: Future {
    next_clone_id: Cell<u64>,
    original_future: RefCell<F>,
    state: RefCell<ForkedPromiseState<F::Item, F::Error>>,
}

enum ForkedPromiseState<T, E> {
    Waiting(Arc<Unparker>),
    Done(Result<T, E>),
}

pub struct ForkedPromise<F> where F: Future {
    id: u64,
    inner: Rc<ForkedPromiseInner<F>>,
}

impl <F> Clone for ForkedPromise<F> where F: Future {
    fn clone(&self) -> ForkedPromise<F> {
        let clone_id = self.inner.next_clone_id.get();
        self.inner.next_clone_id.set(clone_id + 1);
        ForkedPromise {
            id: clone_id,
            inner: self.inner.clone(),
        }
    }
}

impl <F> ForkedPromise<F> where F: Future {
    pub fn new(f: F) -> ForkedPromise<F> {
        ForkedPromise {
            id: 0,
            inner: Rc::new(ForkedPromiseInner {
                next_clone_id: Cell::new(1),
                original_future: RefCell::new(f),
                state: RefCell::new(ForkedPromiseState::Waiting(
                    Arc::new(Unparker::new(AtomicBool::new(true))))),
            })
        }
    }
}

impl<F> Drop for ForkedPromise<F> where F: Future {
    fn drop(&mut self) {
        match *self.inner.state.borrow_mut() {
            ForkedPromiseState::Waiting(ref unparker) => {
                unparker.remove(self.id);
            }
            ForkedPromiseState::Done(_) => (),
        }
    }
}

impl <F> Future for ForkedPromise<F>
    where F: Future, F::Item: Clone, F::Error: Clone,
{
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> ::futures::Poll<Self::Item, Self::Error> {
        let event = match *self.inner.state.borrow_mut() {
            ForkedPromiseState::Waiting(ref unparker) => {
                if !unparker.original_needs_poll.swap(false, Ordering::SeqCst) {
                    unparker.insert(self.id, task::park());
                    return Ok(::futures::Async::NotReady)
                }
                task::UnparkEvent::new(unparker.clone(), 0)
            }
            ForkedPromiseState::Done(ref r) => {
                match *r {
                    Ok(ref v) => return Ok(::futures::Async::Ready(v.clone())),
                    Err(ref e) => return Err(e.clone()),
                }
            }
        };

        let done_val = match task::with_unpark_event(event, || self.inner.original_future.borrow_mut().poll()) {
            Ok(::futures::Async::NotReady) => {
                return Ok(::futures::Async::NotReady)
            }
            Ok(::futures::Async::Ready(v)) => Ok(v),
            Err(e) => Err(e),
        };

        match ::std::mem::replace(
            &mut *self.inner.state.borrow_mut(),
            ForkedPromiseState::Done(done_val.clone()))
        {
            ForkedPromiseState::Waiting(ref unparker) => unparker.unpark(),
            _ => unreachable!(),
        }

        match done_val {
            Ok(v) => Ok(::futures::Async::Ready(v)),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod test {
    use futures::{Future, Poll};
    use futures::sync::oneshot;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::cell::RefCell;
    use super::ForkedPromise;

    enum Mode { Left, Right }

    struct ModedFutureInner<F> where F: Future {
        mode: Mode,
        left: F,
        right: F,
        task: Option<::futures::task::Task>,
    }

    struct ModedFuture<F> where F: Future {
        inner: Rc<RefCell<ModedFutureInner<F>>>,
    }

    struct ModedFutureHandle<F> where F: Future {
        inner: Rc<RefCell<ModedFutureInner<F>>>,
    }

    impl <F> ModedFuture<F> where F: Future {
        pub fn new(left: F, right: F, mode: Mode) -> (ModedFutureHandle<F>, ModedFuture<F>) {
            let inner = Rc::new(RefCell::new(ModedFutureInner {
                left: left, right: right, mode: mode, task: None,
            }));
            (ModedFutureHandle { inner: inner.clone() }, ModedFuture { inner: inner })
        }
    }

    impl <F> ModedFutureHandle<F> where F: Future {
        pub fn switch_mode(&mut self, mode: Mode) {
            self.inner.borrow_mut().mode = mode;
            if let Some(t) = self.inner.borrow_mut().task.take() {
                // The other future may have become ready while we were ignoring it.
                t.unpark();
            }
        }
    }

    impl <F> Future for ModedFuture<F> where F: Future {
        type Item = F::Item;
        type Error = F::Error;
        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            let ModedFutureInner { ref mut mode, ref mut left, ref mut right, ref mut task } =
                *self.inner.borrow_mut();
            *task = Some(::futures::task::park());
            match *mode {
                Mode::Left => left.poll(),
                Mode::Right => right.poll(),
            }
        }
    }

    struct Reaper;
    impl ::task_set::TaskReaper<(), ()> for Reaper {
        fn task_failed(&mut self, _err: ()) {}
    }

    struct Unpark;
    impl ::futures::executor::Unpark for Unpark {
        fn unpark(&self) {}
    }

    fn forked_propagates_unpark_helper(spawn_moded_first: bool) {
        let (handle, tasks) = ::task_set::TaskSet::<(), ()>::new(Box::new(Reaper));

        let mut spawn = ::futures::executor::spawn(tasks);
        let unpark: Arc<::futures::executor::Unpark> = Arc::new(Unpark);

        let (tx, rx) = oneshot::channel::<u32>();
        let f1 = ForkedPromise::new(rx);
        let f2 = f1.clone();

        let (mut mfh, mf) = ModedFuture::new(
            Box::new(f1.map_err(|_| ())) as Box<Future<Item=u32, Error=()>>,
            Box::new(::futures::future::empty()) as Box<Future<Item=u32, Error=()>>,
            Mode::Left);


        let mut handle0 = handle.clone();
        let spawn_mf = move || {
            handle0.add(mf.map(|_| ()));
        };

        let mut handle0 = handle.clone();
        let spawn_f2 = move || {
            let mut handle1 = handle0.clone();
            handle0.add(f2.map(move |_| handle1.terminate(Ok(()))).map_err(|_|()));

        };

        if spawn_moded_first {
            (spawn_mf)();
            match spawn.poll_future(unpark.clone()) {
                Ok(::futures::Async::NotReady) => (),
                _ => panic!("should not be ready yet."),
            }
            (spawn_f2)();
        } else {
            (spawn_f2)();
            match spawn.poll_future(unpark.clone()) {
                Ok(::futures::Async::NotReady) => (),
                _ => panic!("should not be ready yet."),
            }
            (spawn_mf)();
        }

        match spawn.poll_future(unpark.clone()) {
            Ok(::futures::Async::NotReady) => (),
            _ => panic!("should not be ready yet."),
        }

        mfh.switch_mode(Mode::Right);

        tx.complete(11); // This should cause `f2` and then eventually `spawn` to resolve.

        loop {
            match spawn.poll_future(unpark.clone()) {
                Ok(::futures::Async::NotReady) => (),
                Ok(::futures::Async::Ready(_)) => break,
                Err(e) => panic!("error: {:?}", e),
            }
        }
    }

    #[test]
    fn forked_propagates_unpark() {
        forked_propagates_unpark_helper(true);
        forked_propagates_unpark_helper(false);
    }
}
