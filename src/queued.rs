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
                                 ResultsHook, ResultsDoneHook};

use futures::Future;

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use {broken, local, Attach, ForkedPromise};
use sender_queue::SenderQueue;

pub struct PipelineInner {
    // Once the promise resolves, this will become non-null and point to the underlying object.
    redirect: Option<Box<PipelineHook>>,

    // Represents the operation which will set `redirect` when possible.
    self_resolution_op: Promise<(), Error>,

    clients_to_resolve: SenderQueue<(Weak<RefCell<ClientInner>>, Vec<PipelineOp>), ()>,
}

impl PipelineInner {
    fn resolve(this: &Rc<RefCell<PipelineInner>>, result: Result<Box<PipelineHook>, Error>) {
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

pub struct Pipeline {
    inner: Rc<RefCell<PipelineInner>>,
}

impl Pipeline {
    pub fn new(promise_param: Promise<Box<PipelineHook>, Error>) -> Pipeline {
        let inner = Rc::new(RefCell::new(PipelineInner {
            redirect: None,
            self_resolution_op: Promise::ok(()),
            clients_to_resolve: SenderQueue::new(),
        }));

        let this = Rc::downgrade(&inner);
        let self_res = ::eagerly_evaluate(promise_param.then(move |result| {
            let this = match this.upgrade() {
                Some(v) => v,
                None => return Err(Error::failed("dangling self reference in queued::Pipeline".into())),
            };
            PipelineInner::resolve(&this, result);
            Ok(())
        }));
        inner.borrow_mut().self_resolution_op = self_res;
        Pipeline { inner: inner }
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

    resolved: bool,

    // When this promise resolves, each queued call will be forwarded to the real client.  This needs
    // to occur *before* any 'whenMoreResolved()' promises resolve, because we want to make sure
    // previously-queued calls are delivered before any new calls made in response to the resolution.
    call_forwarding_queue: SenderQueue<(u64, u16, Box<ParamsHook>, Box<ResultsHook>,
                                        Promise<Box<ResultsDoneHook>, Error>),
                                       (Promise<(), Error>, Box<PipelineHook>)>,


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
        assert!(!state.borrow().resolved);

        let client = match result {
            Ok(clienthook) => clienthook,
            Err(e) => broken::new_cap(e),
        };
        state.borrow_mut().redirect = Some(client.add_ref());

        for (args, waiter) in state.borrow_mut().call_forwarding_queue.drain() {
            let (interface_id, method_id, params, results, results_done) = args;
            let result_pair = client.call(interface_id, method_id, params, results, results_done);
            waiter.complete(result_pair);
        }

        for ((), waiter) in state.borrow_mut().client_resolution_queue.drain() {
            waiter.complete(client.add_ref());
        }

        state.borrow_mut().pipeline_inner.take();
        state.borrow_mut().resolved = true;
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
            resolved: false,
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

    fn call(&self, interface_id: u64, method_id: u16, params: Box<ParamsHook>, results: Box<ResultsHook>,
            results_done: Promise<Box<ResultsDoneHook>, Error>)
        -> (Promise<(), Error>, Box<PipelineHook>)
    {
        if let Some(ref client) = self.inner.borrow().redirect {
           return client.call(interface_id, method_id, params, results, results_done)
        }

        let inner_clone = self.inner.clone();

        let promise_for_pair = self.inner.borrow_mut().call_forwarding_queue.push(
            (interface_id, method_id, params, results, results_done)).attach(inner_clone);

        let (promise_promise, pipeline_promise) = ::split::split(promise_for_pair);
        let pipeline = Pipeline::new(Promise::from_future(pipeline_promise));
        (Promise::from_future(promise_promise.flatten()), Box::new(pipeline))
    }

    fn get_ptr(&self) -> usize {
        (&*self.inner.borrow()) as * const _ as usize
    }

    fn get_brand(&self) -> usize {
        0
    }

    fn get_resolved(&self) -> Option<Box<ClientHook>> {
        match self.inner.borrow().redirect {
            Some(ref inner) => Some(inner.clone()),
            None => None,
        }
    }

    fn when_more_resolved(&self) -> Option<Promise<Box<ClientHook>, Error>> {
        if let Some(ref client) = self.inner.borrow().redirect {
            return Some(Promise::ok(client.add_ref()));
        }

        Some(self.inner.borrow_mut().client_resolution_queue.push(()))
    }
}
