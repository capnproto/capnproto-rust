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
use std::rc::{Rc};

use {broken, local, ForkedPromise};

struct PipelineInner {
    promise: ForkedPromise<Promise<Box<PipelineHook>, Error>>,

    // Once the promise resolves, this will become non-null and point to the underlying object.
    redirect: Option<Box<PipelineHook>>,

    // Represents the operation which will set `redirect` when possible.
    self_resolution_op: Promise<(), Error>,
}

pub struct Pipeline {
    inner: Rc<RefCell<PipelineInner>>,
}

impl Pipeline {
    pub fn new(promise_param: Promise<Box<PipelineHook>, Error>) -> Pipeline {
        let promise = ForkedPromise::new(promise_param);
        let branch = promise.clone();
        let inner = Rc::new(RefCell::new(PipelineInner {
            promise: promise,
            redirect: None,
            self_resolution_op: Promise::ok(()),
        }));
        let this = Rc::downgrade(&inner);
        let self_res = branch.then(move |result| {
            let this = this.upgrade().expect("dangling reference?");
            match result {
                Ok(pipeline_hook) => {
                    this.borrow_mut().redirect = Some(pipeline_hook);
                }
                Err(e) => {
                    this.borrow_mut().redirect = Some(Box::new(broken::Pipeline::new(e)));
                }
            }
            Ok(())
        });
        inner.borrow_mut().self_resolution_op = Promise::from_future(self_res);
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
        let mut cp = Vec::with_capacity(ops.len());
        for &op in ops {
            cp.push(op);
        }
        self.get_pipelined_cap_move(cp)
    }

    fn get_pipelined_cap_move(&self, ops: Vec<PipelineOp>) -> Box<ClientHook> {
        match &self.inner.borrow().redirect {
            &Some(ref p) => {
                return p.get_pipelined_cap_move(ops)
            }
            &None => (),
        }
        let client_promise = self.inner.borrow_mut().promise.clone().map(move |pipeline| {
            pipeline.get_pipelined_cap_move(ops)
        });

        Box::new(Client::new(Promise::from_future(client_promise)))
    }
}

type ClientHookPromiseFork = ForkedPromise<Promise<Box<ClientHook>, Error>>;

struct ClientInner {
    // Once the promise resolves, this will become non-null and point to the underlying object.
    redirect: Option<Box<ClientHook>>,

    // Promise that resolves when we have a new ClientHook to forward to.
    //
    // This fork shall only have three branches:  `selfResolutionOp`, `promiseForCallForwarding`, and
    // `promiseForClientResolution`, in that order.
    _promise: ClientHookPromiseFork,

    // Represents the operation which will set `redirect` when possible.
    self_resolution_op: Promise<(), Error>,

    // When this promise resolves, each queued call will be forwarded to the real client.  This needs
    // to occur *before* any 'whenMoreResolved()' promises resolve, because we want to make sure
    // previously-queued calls are delivered before any new calls made in response to the resolution.
    promise_for_call_forwarding: ClientHookPromiseFork,

    // whenMoreResolved() returns forks of this promise.  These must resolve *after* queued calls
    // have been initiated (so that any calls made in the whenMoreResolved() handler are correctly
    // delivered after calls made earlier), but *before* any queued calls return (because it might
    // confuse the application if a queued call returns before the capability on which it was made
    // resolves).  Luckily, we know that queued calls will involve, at the very least, an
    // eventLoop.evalLater.
    promise_for_client_resolution: ClientHookPromiseFork,
}

pub struct Client {
    inner: Rc<RefCell<ClientInner>>,
}

impl Client {
    pub fn new(promise_param: Promise<Box<ClientHook>, Error>)
           -> Client
    {
        let promise = ForkedPromise::new(promise_param);
        let branch1 = promise.clone();
        let branch2 = promise.clone();
        let branch3 = promise.clone();
        let inner = Rc::new(RefCell::new(ClientInner {
            redirect: None,
            _promise: promise,
            self_resolution_op: Promise::from_future(::futures::future::empty()),
            promise_for_call_forwarding: ForkedPromise::new(Promise::from_future(branch2)),
            promise_for_client_resolution: ForkedPromise::new(Promise::from_future(branch3)),
        }));
        let this = Rc::downgrade(&inner);
        let self_resolution_op = branch1.then(move |result| {
            let state = this.upgrade().expect("dangling reference to QueuedClient");
            match result {
                Ok(clienthook) => {
                    state.borrow_mut().redirect = Some(clienthook);
                }
                Err(e) => {
                    state.borrow_mut().redirect = Some(broken::new_cap(e));
                }
            }
            Ok(())
        });
        inner.borrow_mut().self_resolution_op = Promise::from_future(self_resolution_op);
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
        // The main reason this is complicated is that we don't want the call to get cancelled
        // if the caller drops just the returned promise but not the pipeline.
        struct CallResultHolder {
            promise: Option<Promise<(), Error>>,
            pipeline: Option<Box<PipelineHook>>,
        }

        let call_result_promise =
            ForkedPromise::new(Box::new(self.inner.borrow_mut().promise_for_call_forwarding.clone().and_then(move |client| {
                let (promise, pipeline) = client.call(interface_id, method_id, params, results, results_done);
                Ok(Rc::new(RefCell::new(CallResultHolder {
                    promise: Some(promise),
                    pipeline: Some(pipeline),
                })))
            })));

        let pipeline_promise = call_result_promise.clone().and_then(|call_result| {
            Ok(call_result.borrow_mut().pipeline.take().expect("pipeline gone?"))
        });
        let pipeline = Pipeline::new(Promise::from_future(pipeline_promise));

        let completion_promise = call_result_promise.clone().and_then(|call_result| {
            call_result.borrow_mut().promise.take().expect("promise gone?")
        });

        (Promise::from_future(completion_promise), Box::new(pipeline))
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
        Some(Promise::from_future(self.inner.borrow_mut().promise_for_client_resolution.clone()))
    }
}
