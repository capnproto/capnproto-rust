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

    // Only ever `Some` when `state` is `Waiting`. You might think that it's a good idea
    // to fold this field into the `State` enum, but then a double-borrow panic is possible:
    // `state` would need to be mutably borrowed when we call `poll()` on the original future,
    // polling the original future can cause clones of the `ForkedPromise` to get dropped,
    // and `ForkedPromise::drop()` needs to borrow `state` again.
    original_future: RefCell<Option<F>>,

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
                original_future: RefCell::new(Some(f)),
                state: RefCell::new(ForkedPromiseState::Waiting(
                    Arc::new(Unparker::new(AtomicBool::new(true))))),
            })
        }
    }
}

impl<F> Drop for ForkedPromise<F> where F: Future {
    fn drop(&mut self) {
        match *self.inner.state.borrow() {
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
                if self.inner.original_future.try_borrow().is_ok() {
                    if unparker.original_needs_poll.swap(false, Ordering::SeqCst) {
                        task::UnparkEvent::new(unparker.clone(), 0)
                    } else {
                        unparker.insert(self.id, task::park());
                        return Ok(::futures::Async::NotReady)
                    }
                } else {
                    // This is recursive call to poll()! Try again later.
                    task::park().unpark();
                    return Ok(::futures::Async::NotReady)
                }
            }
            ForkedPromiseState::Done(ref r) => {
                match *r {
                    Ok(ref v) => return Ok(::futures::Async::Ready(v.clone())),
                    Err(ref e) => return Err(e.clone()),
                }
            }
        };

        let done_val = match *self.inner.original_future.borrow_mut() {
            Some(ref mut original_future) => {
                match task::with_unpark_event(event, || original_future.poll()) {
                    Ok(::futures::Async::NotReady) => {
                        return Ok(::futures::Async::NotReady)
                    }
                    Ok(::futures::Async::Ready(v)) => Ok(v),
                    Err(e) => Err(e),
                }
            }
            None => unreachable!(),
        };

        // No longer need to hold on to the original future.
        self.inner.original_future.borrow_mut().take();

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


    // This test was pilfered from a futures::future::Shared test.
    #[test]
    fn drop_in_poll() {
        let slot = Rc::new(RefCell::new(None));
        let slot2 = slot.clone();
        let future = ForkedPromise::new(::futures::future::poll_fn(move || {
            drop(slot2.borrow_mut().take().unwrap());
            Ok::<_, ()>(1.into())
        }));
        let future2 = Box::new(future.clone()) as Box<Future<Item=_, Error=_>>;
        *slot.borrow_mut() = Some(future2);
        assert_eq!(future.wait().unwrap(), 1);
    }

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

    #[test]
    fn recursive_poll() {
        use futures::sync::mpsc;
        use futures::{Stream, future, task};

        let mut core = local_executor::Core::new();
        let (tx0, rx0) = mpsc::unbounded::<Box<Future<Item=(),Error=()>>>();
        let run_stream = rx0.for_each(|f| f);

        let (tx1, rx1) = oneshot::channel::<()>();

        let f1 = ForkedPromise::new(run_stream);
        let f2 = f1.clone();
        let f3 = f1.clone();
        tx0.send(Box::new(future::lazy(move || {
            task::park().unpark();
            f1.map(|_|()).map_err(|_|())
                .select(rx1.map_err(|_|()))
                .map(|_| ()).map_err(|_|())
        }))).unwrap();

        core.spawn(f2.map(|_|()).map_err(|_|()));

        // Call poll() on the spawned future. We want to be sure that this does not trigger a
        // deadlock or panic due to a recursive lock() on a mutex.
        core.run(future::ok::<(),()>(())).unwrap();

        tx1.complete(()); // Break the cycle.
        drop(tx0);
        core.run(f3).unwrap();
    }


    mod local_executor {
        //! (This module is borrowed from futures-rs/tests/support)
        //!
        //! Execution of futures on a single thread
        //!
        //! This module has no special handling of any blocking operations other than
        //! futures-aware inter-thread communications, and should therefore probably not
        //! be used to manage IO.

        use std::sync::{Arc, Mutex, mpsc};
        use std::collections::HashMap;
        use std::collections::hash_map;
        use std::boxed::Box;
        use std::rc::Rc;
        use std::cell::RefCell;

        use futures::{Future, Async};

        use futures::executor::{self, Spawn};

        /// Main loop object
        pub struct Core {
            unpark_send: mpsc::Sender<u64>,
            unpark: mpsc::Receiver<u64>,
            live: HashMap<u64, Spawn<Box<Future<Item=(), Error=()>>>>,
            next_id: u64,
        }

        impl Core {
            /// Create a new `Core`.
            pub fn new() -> Self {
                let (send, recv) = mpsc::channel();
                Core {
                    unpark_send: send,
                    unpark: recv,
                    live: HashMap::new(),
                    next_id: 0,
                }
            }

            /// Spawn a future to be executed by a future call to `run`.
            pub fn spawn<F>(&mut self, f: F)
                where F: Future<Item=(), Error=()> + 'static
            {
                self.live.insert(self.next_id, executor::spawn(Box::new(f)));
                self.unpark_send.send(self.next_id).unwrap();
                self.next_id += 1;
            }

            /// Run the loop until all futures previously passed to `spawn` complete.
            pub fn _wait(&mut self) {
                while !self.live.is_empty() {
                    self.turn();
                }
            }

            /// Run the loop until the future `f` completes.
            pub fn run<F>(&mut self, f: F) -> Result<F::Item, F::Error>
                where F: Future + 'static,
                      F::Item: 'static,
                      F::Error: 'static,
            {
                let out = Rc::new(RefCell::new(None));
                let out2 = out.clone();
                self.spawn(f.then(move |x| { *out.borrow_mut() = Some(x); Ok(()) }));
                loop {
                    self.turn();
                    if let Some(x) = out2.borrow_mut().take() {
                        return x;
                    }
                }
            }

            fn turn(&mut self) {
                let task = self.unpark.recv().unwrap(); // Safe to unwrap because self.unpark_send keeps the channel alive
                let unpark = Arc::new(Unpark { task: task, send: Mutex::new(self.unpark_send.clone()), });
                let mut task = if let hash_map::Entry::Occupied(x) = self.live.entry(task) { x } else { return };
                let result = task.get_mut().poll_future(unpark);
                match result {
                    Ok(Async::Ready(())) => { task.remove(); }
                    Err(()) => { task.remove(); }
                    Ok(Async::NotReady) => {}
                }
            }
        }

        struct Unpark {
            task: u64,
            send: Mutex<mpsc::Sender<u64>>,
        }

        impl executor::Unpark for Unpark {
            fn unpark(&self) {
                let _ = self.send.lock().unwrap().send(self.task);
            }
        }
    }
}
