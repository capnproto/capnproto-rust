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

use futures::channel::{mpsc, oneshot};
use futures::stream::FuturesUnordered;
use futures::{Future, FutureExt, Stream};
use std::pin::Pin;
use std::task::{Context, Poll};

use std::cell::RefCell;
use std::rc::Rc;

enum EnqueuedTask<E> {
    Task(Pin<Box<dyn Future<Output = Result<(), E>>>>),
    Terminate(Result<(), E>),
    OnEmpty(oneshot::Sender<()>),
}

enum TaskInProgress<E> {
    Task(Pin<Box<dyn Future<Output = ()>>>),
    Terminate(Option<Result<(), E>>),
}

impl<E> Unpin for TaskInProgress<E> {}

enum TaskDone<E> {
    Continue,
    Terminate(Result<(), E>),
}

impl<E> Future for TaskInProgress<E> {
    type Output = TaskDone<E>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        match *self {
            Self::Terminate(ref mut r) => Poll::Ready(TaskDone::Terminate(r.take().unwrap())),
            Self::Task(ref mut f) => match f.as_mut().poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(()) => Poll::Ready(TaskDone::Continue),
            },
        }
    }
}

#[must_use = "a TaskSet does nothing unless polled"]
pub struct TaskSet<E> {
    enqueued: Option<mpsc::UnboundedReceiver<EnqueuedTask<E>>>,
    in_progress: FuturesUnordered<TaskInProgress<E>>,
    on_empty_fulfillers: Vec<oneshot::Sender<()>>,
    reaper: Rc<RefCell<Box<dyn TaskReaper<E>>>>,
}

impl<E> TaskSet<E>
where
    E: 'static,
{
    pub fn new(reaper: Box<dyn TaskReaper<E>>) -> (TaskSetHandle<E>, Self)
    where
        E: 'static,
        E: ::std::fmt::Debug,
    {
        let (sender, receiver) = mpsc::unbounded();

        let set = Self {
            enqueued: Some(receiver),
            in_progress: FuturesUnordered::new(),
            on_empty_fulfillers: vec![],
            reaper: Rc::new(RefCell::new(reaper)),
        };

        // If the FuturesUnordered ever gets empty, its stream will terminate, which
        // is not what we want. So we make sure there is always at least one future in it.
        set.in_progress
            .push(TaskInProgress::Task(Box::pin(::futures::future::pending())));

        let handle = TaskSetHandle { sender };

        (handle, set)
    }

    fn update_on_empty_fulfillers(&mut self) {
        // There is always the one pending() future that we added in `new()`.
        if self.in_progress.len() <= 1 {
            for f in std::mem::take(&mut self.on_empty_fulfillers) {
                let _ = f.send(());
            }
        }
    }
}

#[derive(Clone)]
pub struct TaskSetHandle<E> {
    sender: mpsc::UnboundedSender<EnqueuedTask<E>>,
}

impl<E> TaskSetHandle<E>
where
    E: 'static,
{
    pub fn add<F>(&mut self, f: F)
    where
        F: Future<Output = Result<(), E>> + 'static,
    {
        let _ = self.sender.unbounded_send(EnqueuedTask::Task(Box::pin(f)));
    }

    pub fn terminate(&mut self, result: Result<(), E>) {
        let _ = self.sender.unbounded_send(EnqueuedTask::Terminate(result));
    }

    /// Returns a future that finishes at the next time when the task set
    /// is empty. If the task set is terminated, the oneshot will be canceled.
    pub fn on_empty(&mut self) -> oneshot::Receiver<()> {
        let (s, r) = oneshot::channel();
        let _ = self.sender.unbounded_send(EnqueuedTask::OnEmpty(s));
        r
    }
}

/// For a specific kind of task, `TaskReaper` defines the procedure that should
/// be invoked when it succeeds or fails.
pub trait TaskReaper<E>
where
    E: 'static,
{
    fn task_succeeded(&mut self) {}
    fn task_failed(&mut self, error: E);
}

impl<E> Future for TaskSet<E>
where
    E: 'static,
{
    type Output = Result<(), E>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut enqueued_stream_complete = false;
        if let Self {
            enqueued: Some(ref mut enqueued),
            ref mut in_progress,
            ref reaper,
            ref mut on_empty_fulfillers,
        } = self.as_mut().get_mut()
        {
            loop {
                match Pin::new(&mut *enqueued).poll_next(cx) {
                    Poll::Pending => break,
                    Poll::Ready(None) => {
                        enqueued_stream_complete = true;
                        break;
                    }
                    Poll::Ready(Some(EnqueuedTask::Terminate(r))) => {
                        in_progress.push(TaskInProgress::Terminate(Some(r)));
                    }
                    Poll::Ready(Some(EnqueuedTask::Task(f))) => {
                        let reaper = Rc::downgrade(reaper);
                        in_progress.push(TaskInProgress::Task(Box::pin(f.map(move |r| {
                            match reaper.upgrade() {
                                None => (), // TaskSet must have been dropped.
                                Some(rc_reaper) => match r {
                                    Ok(()) => rc_reaper.borrow_mut().task_succeeded(),
                                    Err(e) => rc_reaper.borrow_mut().task_failed(e),
                                },
                            }
                        }))));
                    }
                    Poll::Ready(Some(EnqueuedTask::OnEmpty(f))) => {
                        on_empty_fulfillers.push(f);
                    }
                }
            }
        }
        if enqueued_stream_complete {
            drop(self.enqueued.take());
        }

        loop {
            match Stream::poll_next(Pin::new(&mut self.in_progress), cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(v) => match v {
                    None => return Poll::Ready(Ok(())),
                    Some(TaskDone::Continue) => self.update_on_empty_fulfillers(),
                    Some(TaskDone::Terminate(Ok(()))) => {
                        self.on_empty_fulfillers.clear();
                        return Poll::Ready(Ok(()));
                    }
                    Some(TaskDone::Terminate(Err(e))) => {
                        self.on_empty_fulfillers.clear();
                        return Poll::Ready(Err(e));
                    }
                },
            }
        }
    }
}
