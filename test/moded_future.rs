use futures::{Future, Poll};
use std::rc::Rc;
use std::cell::RefCell;

pub enum Mode { Left, Right }

pub struct ModedFutureInner<F> where F: Future {
    mode: Mode,
    left: F,
    right: F,
    task: Option<::futures::task::Task>,
}

pub struct ModedFuture<F> where F: Future {
    inner: Rc<RefCell<ModedFutureInner<F>>>,
}

pub struct ModedFutureHandle<F> where F: Future {
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
