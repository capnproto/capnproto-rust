// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
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

//! Hooks for for the RPC system.
//!
//! Roughly corresponds to capability.h in the C++ implementation.

use alloc::boxed::Box;
use core::future::{Future};
use core::pin::{Pin};
use core::marker::{PhantomData, Unpin};
use core::task::Poll;
#[cfg(feature = "rpc_try")]
use core::ops::Try;

use crate::{any_pointer, Error, MessageSize};
use crate::traits::{Pipelined, Owned};
use crate::private::capability::{ClientHook, ParamsHook, RequestHook, ResponseHook, ResultsHook};

/// A computation that might eventually resolve to a value of type `T` or to an error
///  of type `E`. Dropping the promise cancels the computation.
#[must_use = "futures do nothing unless polled"]
pub struct Promise<T, E> {
    inner: PromiseInner<T, E>,
}

enum PromiseInner<T, E> {
    Immediate(Result<T,E>),
    Deferred(Pin<Box<dyn Future<Output=core::result::Result<T,E>> + 'static>>),
    Empty,
}

// Allow Promise<T,E> to be Unpin, regardless of whether T and E are.
impl <T, E> Unpin for PromiseInner<T,E> {}

impl <T, E> Promise<T, E> {
    pub fn ok(value: T) -> Promise<T, E> {
        Promise { inner: PromiseInner::Immediate(Ok(value)) }
    }

    pub fn err(error: E) -> Promise<T, E> {
        Promise { inner: PromiseInner::Immediate(Err(error)) }
    }

    pub fn from_future<F>(f: F) -> Promise<T, E>
        where F: Future<Output=core::result::Result<T,E>> + 'static
    {
        Promise { inner: PromiseInner::Deferred(Box::pin(f)) }
    }
}

impl <T, E> Future for Promise<T, E>
{
    type Output = core::result::Result<T,E>;

    fn poll(self: Pin<&mut Self>, cx: &mut ::core::task::Context) -> Poll<Self::Output> {
        match self.get_mut().inner {
            PromiseInner::Empty => panic!("Promise polled after done."),
            ref mut imm @ PromiseInner::Immediate(_) => {
                match core::mem::replace(imm, PromiseInner::Empty) {
                    PromiseInner::Immediate(r) => Poll::Ready(r),
                    _ => unreachable!(),
                }
            }
            PromiseInner::Deferred(ref mut f) => f.as_mut().poll(cx),
        }
    }
}

#[cfg(feature = "rpc_try")]
impl<T> std::ops::Try for Promise<T, crate::Error> {
    type Output = Promise<T, crate::Error>;
    type Residual = Result<std::convert::Infallible, crate::Error>;

    fn from_output(output: Self::Output) -> Self {
        output
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        unimplemented!();
    }
}

#[cfg(feature = "rpc_try")]
impl<T> std::ops::FromResidual for Promise<T, crate::Error> {
    fn from_residual(residual: <Self as Try>::Residual) -> Self {
        match residual {
            Ok(_) => unimplemented!(),
            Err(e) => Promise::err(e),
        }
    }
}

/// A promise for a result from a method call.
#[must_use]
pub struct RemotePromise<Results> where Results: Pipelined + for<'a> Owned<'a> + 'static {
    pub promise: Promise<Response<Results>, crate::Error>,
    pub pipeline: Results::Pipeline,
}

/// A response from a method call, as seen by the client.
pub struct Response<Results> {
    pub marker: PhantomData<Results>,
    pub hook: Box<dyn ResponseHook>,
}

impl <Results> Response<Results>
    where Results: Pipelined + for<'a> Owned<'a>
{
    pub fn new(hook: Box<dyn ResponseHook>) -> Response<Results> {
        Response { marker: PhantomData, hook }
    }
    pub fn get(&self) -> crate::Result<<Results as Owned<'_>>::Reader> {
        self.hook.get()?.get_as()
    }
}

/// A method call that has not been sent yet.
pub struct Request<Params, Results> {
    pub marker: PhantomData<(Params, Results)>,
    pub hook: Box<dyn RequestHook>
}

impl <Params, Results> Request<Params, Results>
    where Params: for<'a> Owned<'a>
{
    pub fn new(hook: Box<dyn RequestHook>) -> Request <Params, Results> {
        Request { hook, marker: PhantomData }
    }

    pub fn get(&mut self) -> <Params as Owned<'_>>::Builder {
        self.hook.get().get_as().unwrap()
    }

    pub fn set(&mut self, from: <Params as Owned>::Reader) -> crate::Result<()> {
        self.hook.get().set_as(from)
    }
}

impl <Params, Results> Request <Params, Results>
where Results: Pipelined + for<'a> Owned<'a> + 'static + Unpin,
      <Results as Pipelined>::Pipeline: FromTypelessPipeline
{
    pub fn send(self) -> RemotePromise<Results> {
        let RemotePromise {promise, pipeline, ..} = self.hook.send();
        let typed_promise = Promise::from_future(
            async move {
                Ok(Response {hook: promise.await?.hook,
                             marker: PhantomData})
            });
        RemotePromise { promise: typed_promise,
                        pipeline: FromTypelessPipeline::new(pipeline)
                      }
    }
}

/// The values of the parameters passed to a method call, as seen by the server.
pub struct Params<T> {
    pub marker: PhantomData<T>,
    pub hook: Box<dyn ParamsHook>,
}

impl <T> Params <T> {
    pub fn new(hook: Box<dyn ParamsHook>) -> Params<T> {
        Params { marker: PhantomData, hook }
    }
    pub fn get<'a>(&'a self) -> crate::Result<<T as Owned<'a>>::Reader>
        where T: Owned<'a>
    {
        self.hook.get()?.get_as()
    }
}

/// The return values of a method, written in-place by the method body.
pub struct Results<T> {
    pub marker: PhantomData<T>,
    pub hook: Box<dyn ResultsHook>,
}

impl <T> Results<T>
    where T: for<'a> Owned<'a>
{
    pub fn new(hook: Box<dyn ResultsHook>) -> Results<T> {
        Results { marker: PhantomData, hook }
    }

    pub fn get(&mut self) -> <T as Owned<'_>>::Builder {
        self.hook.get().unwrap().get_as().unwrap()
    }

    pub fn set(&mut self, other: <T as Owned>::Reader) -> crate::Result<()>
    {
        self.hook.get().unwrap().set_as(other)
    }
}

pub trait FromTypelessPipeline {
    fn new (typeless: any_pointer::Pipeline) -> Self;
}

pub trait FromClientHook {
    fn new(hook: Box<dyn ClientHook>) -> Self;
}

/// An untyped client.
pub struct Client {
    pub hook: Box<dyn ClientHook>
}

impl Client {
    pub fn new(hook: Box<dyn ClientHook>) -> Client {
        Client { hook }
    }

    pub fn new_call<Params, Results>(&self,
                                     interface_id : u64,
                                     method_id : u16,
                                     size_hint : Option<MessageSize>)
                                     -> Request<Params, Results> {
        let typeless = self.hook.new_call(interface_id, method_id, size_hint);
        Request { hook: typeless.hook, marker: PhantomData }
    }

    /// If the capability is actually only a promise, the returned promise resolves once the
    /// capability itself has resolved to its final destination (or propagates the exception if
    /// the capability promise is rejected).  This is mainly useful for error-checking in the case
    /// where no calls are being made.  There is no reason to wait for this before making calls; if
    /// the capability does not resolve, the call results will propagate the error.
    pub fn when_resolved(&self) -> Promise<(), Error> {
        self.hook.when_resolved()
    }
}

/// An untyped server.
pub trait Server {
    fn dispatch_call(&mut self, interface_id: u64, method_id: u16,
                     params: Params<any_pointer::Owned>,
                     results: Results<any_pointer::Owned>)
                     -> Promise<(), Error>;
}

/// Trait to track the relationship between generated Server traits and Client structs.
pub trait FromServer<S> : FromClientHook {
    // Implemented by the generated ServerDispatch struct.
    type Dispatch: Server + 'static + core::ops::DerefMut<Target=S>;

    fn from_server(s: S) -> Self::Dispatch;
}
