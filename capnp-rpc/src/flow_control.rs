use capnp::capability::Promise;
use capnp::Error;

use futures::channel::oneshot;
use futures::TryFutureExt;
use std::cell::RefCell;
use std::rc::Rc;

use crate::task_set::{TaskReaper, TaskSet, TaskSetHandle};

pub const DEFAULT_WINDOW_SIZE: usize = 65536;

enum State {
    Running(Vec<oneshot::Sender<Result<(), Error>>>),
    Failed(Error),
}

struct FixedWindowFlowControllerInner {
    window_size: usize,
    in_flight: usize,
    max_message_size: usize,
    state: State,
    empty_fulfiller: Option<oneshot::Sender<Promise<(), Error>>>,
}

impl FixedWindowFlowControllerInner {
    fn is_ready(&self) -> bool {
        // We extend the window by maxMessageSize to avoid a pathological situation when a message
        // is larger than the window size. Otherwise, after sending that message, we would end up
        // not sending any others until the ack was received, wasting a round trip's worth of
        // bandwidth.

        self.in_flight < self.window_size + self.max_message_size
    }
}

pub struct FixedWindowFlowController {
    inner: Rc<RefCell<FixedWindowFlowControllerInner>>,
    tasks: TaskSetHandle<Error>,
}

struct Reaper {
    inner: Rc<RefCell<FixedWindowFlowControllerInner>>,
}

impl TaskReaper<Error> for Reaper {
    fn task_failed(&mut self, error: Error) {
        let mut inner = self.inner.borrow_mut();
        if let State::Running(ref mut blocked_sends) = &mut inner.state {
            for s in std::mem::take(blocked_sends) {
                let _ = s.send(Err(error.clone()));
            }
            inner.state = State::Failed(error)
        }
    }
}

impl FixedWindowFlowController {
    pub fn new(window_size: usize) -> (Self, Promise<(), Error>) {
        let inner = FixedWindowFlowControllerInner {
            window_size,
            in_flight: 0,
            max_message_size: 0,
            state: State::Running(vec![]),
            empty_fulfiller: None,
        };
        let inner = Rc::new(RefCell::new(inner));
        let (tasks, task_future) = TaskSet::new(Box::new(Reaper {
            inner: inner.clone(),
        }));
        (Self { inner, tasks }, Promise::from_future(task_future))
    }
}

impl crate::FlowController for FixedWindowFlowController {
    fn send(
        &mut self,
        message: Box<dyn crate::OutgoingMessage>,
        ack: Promise<(), Error>,
    ) -> Promise<(), Error> {
        let size = message.size_in_words() * 8;
        {
            let mut inner = self.inner.borrow_mut();
            let prev_max_size = inner.max_message_size;
            inner.max_message_size = usize::max(size, prev_max_size);

            // We are REQUIRED to send the message NOW to maintain correct ordering.
            let _ = message.send();

            inner.in_flight += size;
        }
        let inner = self.inner.clone();
        let mut tasks = self.tasks.clone();
        self.tasks.add(async move {
            ack.await?;
            let mut inner = inner.borrow_mut();
            inner.in_flight -= size;
            let is_ready = inner.is_ready();
            match inner.state {
                State::Running(ref mut blocked_sends) => {
                    if is_ready {
                        for s in std::mem::take(blocked_sends) {
                            let _ = s.send(Ok(()));
                        }
                    }

                    if inner.in_flight == 0 {
                        if let Some(f) = inner.empty_fulfiller.take() {
                            let _ = f.send(Promise::from_future(
                                tasks.on_empty().map_err(crate::canceled_to_error),
                            ));
                        }
                    }
                }
                State::Failed(_) => {
                    // A previous call failed, but this one -- which was already in-flight at the
                    // time -- ended up succeeding. That may indicate that the server side is not
                    // properly handling streaming error propagation. Nothing much we can do about
                    // it here though.
                }
            }
            Ok(())
        });

        let mut inner = self.inner.borrow_mut();
        let is_ready = inner.is_ready();
        match inner.state {
            State::Running(ref mut blocked_sends) => {
                if is_ready {
                    Promise::ok(())
                } else {
                    let (snd, rcv) = oneshot::channel();
                    blocked_sends.push(snd);
                    Promise::from_future(async {
                        match rcv.await {
                            Ok(r) => r,
                            Err(e) => Err(crate::canceled_to_error(e)),
                        }
                    })
                }
            }
            State::Failed(ref e) => Promise::err(e.clone()),
        }
    }

    fn wait_all_acked(&mut self) -> Promise<(), Error> {
        let mut inner = self.inner.borrow_mut();
        if let State::Running(ref blocked_sends) = inner.state {
            if !blocked_sends.is_empty() {
                let (snd, rcv) = oneshot::channel();
                inner.empty_fulfiller = Some(snd);
                return Promise::from_future(async move {
                    match rcv.await {
                        Ok(r) => r.await,
                        Err(e) => Err(crate::canceled_to_error(e)),
                    }
                });
            }
        }
        Promise::from_future(self.tasks.on_empty().map_err(crate::canceled_to_error))
    }
}
