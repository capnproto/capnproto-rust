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
                                 RequestHook, ResponseHook, ResultsHook, ResultsDoneHook};

use {ForkedPromise, Attach};
use futures::Future;
use futures::sync::oneshot;

use std::cell::RefCell;
use std::rc::{Rc};

pub struct Response {
    results: Box<ResultsDoneHook>
}

impl Response {
    fn new(results: Box<ResultsDoneHook>) -> Response {
        Response {
            results: results
        }
    }
}

impl ResponseHook for Response {
    fn get<'a>(&'a self) -> ::capnp::Result<any_pointer::Reader<'a>> {
        self.results.get()
    }
}

struct Params {
    request: ::capnp::message::Builder<::capnp::message::HeapAllocator>,
}

impl Params {
    fn new(request: ::capnp::message::Builder<::capnp::message::HeapAllocator>)
           -> Params
    {
        Params {
            request: request,
        }
    }
}

impl ParamsHook for Params {
    fn get<'a>(&'a self) -> ::capnp::Result<any_pointer::Reader<'a>> {
        Ok(try!(self.request.get_root_as_reader()))
    }
}

type HeapMessage = ::capnp::message::Builder<::capnp::message::HeapAllocator>;

struct Results {
    message: Option<HeapMessage>,
    results_done_fulfiller: Option<::futures::sync::oneshot::Sender<Box<ResultsDoneHook>>>,
}

impl Results {
    fn new(fulfiller: ::futures::sync::oneshot::Sender<Box<ResultsDoneHook>>) -> Results {
        Results {
            message: Some(::capnp::message::Builder::new_default()),
            results_done_fulfiller: Some(fulfiller),
        }
    }
}

impl Drop for Results {
    fn drop(&mut self) {
        if let (Some(message), Some(fulfiller)) = (self.message.take(), self.results_done_fulfiller.take()) {
            fulfiller.complete(Box::new(ResultsDone::new(message)));
        } else {
            unreachable!()
        }
    }
}

impl ResultsHook for Results {
    fn get<'a>(&'a mut self) -> ::capnp::Result<any_pointer::Builder<'a>> {
        if let Some(ref mut message) = self.message {
            Ok(try!(message.get_root()))
        } else {
            unreachable!()
        }
    }

    fn tail_call(self: Box<Self>, _request: Box<RequestHook>) -> Promise<(), Error> {
        unimplemented!()
    }

    fn direct_tail_call(self: Box<Self>, _request: Box<RequestHook>)
                        -> (Promise<(), Error>, Box<PipelineHook>)
    {
        unimplemented!()
    }

    fn allow_cancellation(&self) {
        unimplemented!()
    }
}

struct ResultsDoneInner {
    message: ::capnp::message::Builder<::capnp::message::HeapAllocator>
}

struct ResultsDone {
    inner: Rc<ResultsDoneInner>,
}

impl ResultsDone {
    fn new(message: ::capnp::message::Builder<::capnp::message::HeapAllocator>)
        -> ResultsDone
    {
        ResultsDone {
            inner: Rc::new(ResultsDoneInner {
                message: message,
            }),
        }
    }
}

impl ResultsDoneHook for ResultsDone {
    fn add_ref(&self) -> Box<ResultsDoneHook> {
        Box::new(ResultsDone { inner: self.inner.clone() })
    }
    fn get<'a>(&'a self) -> ::capnp::Result<any_pointer::Reader<'a>> {
        Ok(try!(self.inner.message.get_root_as_reader()))
    }
}


pub struct Request {
    message: ::capnp::message::Builder<::capnp::message::HeapAllocator>,
    interface_id: u64,
    method_id: u16,
    client: Box<ClientHook>,
}

impl Request {
    pub fn new(interface_id: u64, method_id: u16,
           _size_hint: Option<::capnp::MessageSize>,
           client: Box<ClientHook>)
           -> Request
    {
        Request {
            message: ::capnp::message::Builder::new_default(),
            interface_id: interface_id,
            method_id: method_id,
            client: client,
        }
    }
}

impl RequestHook for Request {
    fn get<'a>(&'a mut self) -> any_pointer::Builder<'a> {
        self.message.get_root().unwrap()
    }
    fn get_brand(&self) -> usize {
        0
    }
    fn send<'a>(self: Box<Self>) -> ::capnp::capability::RemotePromise<any_pointer::Owned> {
        let tmp = *self;
        let Request { message, interface_id, method_id, client } = tmp;
        let params = Params::new(message);
        let (results_done_fulfiller, results_done_promise) = oneshot::channel::<Box<ResultsDoneHook>>();
        let results_done_promise = results_done_promise.map_err(|e| e.into());

        let forked_results_done = ForkedPromise::new(results_done_promise);
        let results = Results::new(results_done_fulfiller);

        let (promise, pipeline) = client.call(interface_id, method_id,
                                              Box::new(params), Box::new(results),
                                              Promise::from_future(forked_results_done.clone()));

        let results_done_branch2 = forked_results_done.clone();

        // Fork so that dropping just the returned promise doesn't cancel the call.
        let forked = ForkedPromise::new(promise);

        let promise = forked.clone().and_then(move |()| {
            results_done_branch2.and_then(|results_done_hook| {
                Ok(::capnp::capability::Response::new(Box::new(Response::new(results_done_hook))))
            })
        });

        let pipeline_promise = Promise::from_future(forked.clone().map(move |_| pipeline));
        let pipeline = any_pointer::Pipeline::new(Box::new(::queued::Pipeline::new(pipeline_promise)));

        ::capnp::capability::RemotePromise {
            promise: Promise::from_future(promise),
            pipeline: pipeline,
        }
    }
    fn tail_send(self: Box<Self>)
                 -> Option<(u32, Promise<(), Error>, Box<PipelineHook>)>
    {
        unimplemented!()
    }
}

struct PipelineInner {
    results: Box<ResultsDoneHook>,
}

pub struct Pipeline {
    inner: Rc<RefCell<PipelineInner>>,
}

impl Pipeline {
    pub fn new(results: Box<ResultsDoneHook>) -> Pipeline {
        Pipeline {
            inner: Rc::new(RefCell::new(PipelineInner { results: results }))
        }
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
        // Do I need to call imbue() here?
        // yeah, probably.

        match self.inner.borrow_mut().results.get().unwrap().get_pipelined_cap(ops) {
            Ok(v) => v,
            Err(e) => Box::new(::broken::Client::new(e, true, 0)) as Box<ClientHook>,
        }
    }
}

struct ClientInner {
    server: Box<::capnp::capability::Server>,
}

pub struct Client {
    inner: Rc<RefCell<ClientInner>>,
}

impl Client {
    pub fn new(server: Box<::capnp::capability::Server>) -> Client {
        Client {
            inner: Rc::new(RefCell::new(ClientInner { server: server }))
        }
    }
}

impl Clone for Client {
    fn clone(&self) -> Client {
        Client { inner: self.inner.clone() }
    }
}

impl ClientHook for Client {
    fn add_ref(&self) -> Box<ClientHook> {
        Box::new(self.clone())
    }
    fn new_call(&self, interface_id: u64, method_id: u16,
                size_hint: Option<::capnp::MessageSize>)
                -> ::capnp::capability::Request<any_pointer::Owned, any_pointer::Owned>
    {
        ::capnp::capability::Request::new(
            Box::new(Request::new(interface_id, method_id, size_hint, self.add_ref())))
    }

    fn call(&self, interface_id: u64, method_id: u16, params: Box<ParamsHook>, results: Box<ResultsHook>,
            results_done: Promise<Box<ResultsDoneHook>, Error>)
        -> (Promise<(), Error>, Box<PipelineHook>)
    {
        // We don't want to actually dispatch the call synchronously, because we don't want the callee
        // to have any side effects before the promise is returned to the caller.  This helps avoid
        // race conditions.
        let inner = self.inner.clone();
        let promise = ::futures::future::ok(()).and_then(move |()| {
            let server = &mut inner.borrow_mut().server;
            server.dispatch_call(interface_id, method_id,
                                 ::capnp::capability::Params::new(params),
                                 ::capnp::capability::Results::new(results))
        }).attach(self.add_ref());

        let forked = ForkedPromise::new(promise);

        let branch = forked.clone();
        let pipeline_promise = results_done.and_then(move |results_done_hook| {
            branch.map(move |()| {
                Box::new(Pipeline::new(results_done_hook)) as Box<PipelineHook>
            })
        });

        let pipeline = Box::new(::queued::Pipeline::new(Promise::from_future(pipeline_promise)));
        let completion_promise = forked;

        (Promise::from_future(completion_promise), pipeline)
    }

    fn get_ptr(&self) -> usize {
        (&*self.inner.borrow()) as * const _ as usize
    }

    fn get_brand(&self) -> usize {
        0
    }

    fn get_resolved(&self) -> Option<Box<ClientHook>> {
        None
    }

    fn when_more_resolved(&self) -> Option<Promise<Box<ClientHook>, Error>> {
        None
    }
}
