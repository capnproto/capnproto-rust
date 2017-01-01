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

use capnp::{any_pointer};
use capnp::Error;
use capnp::capability::Promise;
use capnp::private::capability::{ClientHook, ParamsHook, PipelineHook, PipelineOp,
                                 ResultsHook};

use futures::Future;

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use {broken, local, Attach};
use forked_promise::ForkedPromise;
use sender_queue::SenderQueue;

pub struct PipelineInner {
    // Once the promise resolves, this will become non-null and point to the underlying object.
    redirect: Option<Box<PipelineHook>>,

    promise_to_drive: ForkedPromise<Promise<(), Error>>,

    clients_to_resolve: SenderQueue<(Weak<RefCell<ClientInner>>, Vec<PipelineOp>), ()>,
}

impl PipelineInner {
    fn resolve(this: &Rc<RefCell<PipelineInner>>, result: Result<Box<PipelineHook>, Error>) {
        assert!(this.borrow().redirect.is_none());
        let pipeline = match result {
            Ok(pipeline_hook) => pipeline_hook,
            Err(e) => Box::new(broken::Pipeline::new(e)),
        };

        this.borrow_mut().redirect = Some(pipeline.add_ref());

        for ((weak_client, ops), waiter) in this.borrow_mut().clients_to_resolve.drain() {
            if let Some(client) = weak_client.upgrade() {
                let clienthook = pipeline.get_pipelined_cap_move(ops);
                ClientInner::resolve(&client, Ok(clienthook));
            }
            waiter.complete(());
        }
    }
}

pub struct PipelineInnerSender {
    inner: Option<Weak<RefCell<PipelineInner>>>,
}

impl Drop for PipelineInnerSender {
    fn drop(&mut self) {
        if let Some(weak_queued) = self.inner.take() {
            if let Some(pipeline_inner) = weak_queued.upgrade() {
                PipelineInner::resolve(
                    &pipeline_inner,
                    Ok(Box::new(
                        ::broken::Pipeline::new(Error::failed("PipelineInnerSender was canceled".into())))));
            }
        }
    }
}

impl PipelineInnerSender {
    pub fn complete(mut self, pipeline: Box<PipelineHook>) {
        if let Some(weak_queued) = self.inner.take() {
            if let Some(pipeline_inner) = weak_queued.upgrade() {
                ::queued::PipelineInner::resolve(&pipeline_inner, Ok(pipeline));
            }
        }
    }
}

pub struct Pipeline {
    inner: Rc<RefCell<PipelineInner>>,
}

impl Pipeline {
    pub fn new() -> (PipelineInnerSender, Pipeline) {
        let inner = Rc::new(RefCell::new(PipelineInner {
            redirect: None,
            promise_to_drive: ForkedPromise::new(Promise::ok(())),
            clients_to_resolve: SenderQueue::new(),
        }));


        (PipelineInnerSender { inner: Some(Rc::downgrade(&inner)) }, Pipeline { inner: inner })
    }

    pub fn drive<F>(&mut self, promise: F)
        where F: Future<Item=(), Error=Error> + 'static
    {
        self.inner.borrow_mut().promise_to_drive = ForkedPromise::new(Promise::from_future(promise));
    }
}

impl Clone for Pipeline {
    fn clone(&self) -> Pipeline {
        Pipeline { inner: self.inner.clone() }
    }
}

impl PipelineHook for Pipeline {
    fn add_ref(&self) -> Box<PipelineHook> {
        Box::new(self.clone())
    }
    fn get_pipelined_cap(&self, ops: &[PipelineOp]) -> Box<ClientHook> {
        self.get_pipelined_cap_move(ops.into())
    }

    fn get_pipelined_cap_move(&self, ops: Vec<PipelineOp>) -> Box<ClientHook> {
        match &self.inner.borrow().redirect {
            &Some(ref p) => {
                return p.get_pipelined_cap_move(ops)
            }
            &None => (),
        }

        let queued_client = Client::new(Some(self.inner.clone()));
        let weak_queued = Rc::downgrade(&queued_client.inner);
        self.inner.borrow_mut().clients_to_resolve.push_detach((weak_queued, ops));

        Box::new(queued_client)
    }
}

pub struct ClientInner {
    // Once the promise resolves, this will become non-null and point to the underlying object.
    redirect: Option<Box<ClientHook>>,

    pipeline_inner: Option<Rc<RefCell<PipelineInner>>>,

    // When this promise resolves, each queued call will be forwarded to the real client.  This needs
    // to occur *before* any 'whenMoreResolved()' promises resolve, because we want to make sure
    // previously-queued calls are delivered before any new calls made in response to the resolution.
    call_forwarding_queue: SenderQueue<(u64, u16, Box<ParamsHook>, Box<ResultsHook>),
                                       (Promise<(), Error>)>,


    // whenMoreResolved() returns forks of this promise.  These must resolve *after* queued calls
    // have been initiated (so that any calls made in the whenMoreResolved() handler are correctly
    // delivered after calls made earlier), but *before* any queued calls return (because it might
    // confuse the application if a queued call returns before the capability on which it was made
    // resolves).  Luckily, we know that queued calls will involve, at the very least, an
    // eventLoop.evalLater.
    client_resolution_queue: SenderQueue<(), Box<ClientHook>>,
}

impl ClientInner {
    pub fn resolve(state: &Rc<RefCell<ClientInner>>, result: Result<Box<ClientHook>, Error>) {
        assert!(state.borrow().redirect.is_none());
        let client = match result {
            Ok(clienthook) => clienthook,
            Err(e) => broken::new_cap(e),
        };
        state.borrow_mut().redirect = Some(client.add_ref());
        for (args, waiter) in state.borrow_mut().call_forwarding_queue.drain() {
            let (interface_id, method_id, params, results) = args;
            let result_promise = client.call(interface_id, method_id, params, results);
            waiter.complete(result_promise);
        }

        for ((), waiter) in state.borrow_mut().client_resolution_queue.drain() {
            waiter.complete(client.add_ref());
        }
        state.borrow_mut().pipeline_inner.take();
    }
}

pub struct Client {
    pub inner: Rc<RefCell<ClientInner>>,
}

impl Client {
    pub fn new(pipeline_inner: Option<Rc<RefCell<PipelineInner>>>) -> Client
    {
        let inner = Rc::new(RefCell::new(ClientInner {
            pipeline_inner: pipeline_inner,
            redirect: None,
            call_forwarding_queue: SenderQueue::new(),
            client_resolution_queue: SenderQueue::new(),
        }));
        Client {
            inner: inner
        }
    }
}

impl ClientHook for Client {
    fn add_ref(&self) -> Box<ClientHook> {
        Box::new(Client {inner: self.inner.clone()})
    }
    fn new_call(&self, interface_id: u64, method_id: u16,
                size_hint: Option<::capnp::MessageSize>)
                -> ::capnp::capability::Request<any_pointer::Owned, any_pointer::Owned>
    {
        ::capnp::capability::Request::new(
            Box::new(local::Request::new(interface_id, method_id, size_hint, self.add_ref())))
    }

    fn call(&self, interface_id: u64, method_id: u16, params: Box<ParamsHook>, results: Box<ResultsHook>)
        -> Promise<(), Error>
    {
        if let Some(ref client) = self.inner.borrow().redirect {
           return client.call(interface_id, method_id, params, results)
        }

        let inner_clone = self.inner.clone();
        let mut promise = Promise::from_future(self.inner.borrow_mut().call_forwarding_queue.push(
            (interface_id, method_id, params, results)).attach(inner_clone).flatten());

        if let Some(ref pipeline_inner) = self.inner.borrow().pipeline_inner {
            promise = Promise::from_future(
                pipeline_inner.borrow().promise_to_drive.clone().join(promise).map(|v| v.1));
        }

        promise
    }

    fn get_ptr(&self) -> usize {
        (&*self.inner.borrow()) as * const _ as usize
    }

    fn get_brand(&self) -> usize {
        0
    }

    fn get_resolved(&self) -> Option<Box<ClientHook>> {
        match self.inner.borrow().redirect {
            Some(ref inner) => {
                Some(inner.clone())
            }
            None => {
                None
            }
        }
    }

    fn when_more_resolved(&self) -> Option<Promise<Box<ClientHook>, Error>> {
        if let Some(ref client) = self.inner.borrow().redirect {
            return Some(Promise::ok(client.add_ref()));
        }

        let maybe_resolution_op =
            match self.inner.borrow().pipeline_inner {
                Some(ref x) => Some(x.borrow().promise_to_drive.clone()),
                None => None,
            };

       match maybe_resolution_op {
            Some(res_op) => {
                Some(Promise::from_future(
                    res_op.join(self.inner.borrow_mut().client_resolution_queue.push(())).map(|v| v.1)))
            }
            None => {
                Some(self.inner.borrow_mut().client_resolution_queue.push(()))
            }
        }
    }
}
