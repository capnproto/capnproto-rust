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

use any_pointer;
use private::capability::{ClientHook, ParamsHook, RequestHook, ResponseHook, ResultsHook};

#[cfg(feature = "rpc")]
pub type Promise<T,E> = ::gj::Promise<T,E>;

/// This fake Promise struct is defined so that the generated code for interfaces
/// can typecheck even if the "rpc" feature is not enabled.
#[cfg(not(feature = "rpc"))]
pub struct Promise<T,E>(::std::result::Result<T,E>);

#[cfg(not(feature = "rpc"))]
impl <T,E> Promise<T,E> {
    pub fn ok(v: T) -> Promise<T,E> {
        Promise(Ok(v))
    }
    pub fn err(e: E) -> Promise<T,E> {
        Promise(Err(e))
    }
    pub fn map<F, R>(self, _func: F) -> Promise<R, E>
        where F: FnOnce(T) -> Result<R, E>,
    {
        unimplemented!()
    }
}

#[must_use]
pub struct RemotePromise<Results> where Results: ::traits::Pipelined + for<'a> ::traits::Owned<'a> + 'static {
    pub promise: Promise<Response<Results>, ::Error>,
    pub pipeline: Results::Pipeline,
}

pub struct ReaderCapTable {
    pub hooks: Vec<Option<Box<ClientHook>>>
}

impl ReaderCapTable {
    pub fn new(hooks: Vec<Option<Box<ClientHook>>>) -> ReaderCapTable {
        ReaderCapTable { hooks: hooks }
    }

    // Do I need an Imbueable trait?
    pub fn imbue<'a, T>(&'a self) -> T {
        &self.hooks;
        unimplemented!();
    }
}

pub struct Response<Results> {
    pub marker: ::std::marker::PhantomData<Results>,
    pub hook: Box<ResponseHook>,
}

impl <Results> Response<Results>
    where Results: ::traits::Pipelined + for<'a> ::traits::Owned<'a>
{
    pub fn new(hook: Box<ResponseHook>) -> Response<Results> {
        Response { marker: ::std::marker::PhantomData, hook: hook }
    }
    pub fn get<'a>(&'a self) -> ::Result<<Results as ::traits::Owned<'a>>::Reader> {
        try!(self.hook.get()).get_as()
    }
}

pub struct Request<Params, Results> {
    pub marker: ::std::marker::PhantomData<(Params, Results)>,
    pub hook: Box<RequestHook>
}

impl <Params, Results> Request <Params, Results>
    where Params: for<'a> ::traits::Owned<'a>
{
    pub fn new(hook: Box<RequestHook>) -> Request <Params, Results> {
        Request { hook: hook, marker: ::std::marker::PhantomData }
    }

    pub fn get<'a>(&'a mut self) -> <Params as ::traits::Owned<'a>>::Builder {
        self.hook.get().get_as().unwrap()
    }

    pub fn set(&mut self, from: <Params as ::traits::Owned>::Reader) -> ::Result<()> {
        self.hook.get().set_as(from)
    }
}

#[cfg(feature = "rpc")]
impl <Params, Results> Request <Params, Results>
where Results: ::traits::Pipelined + for<'a> ::traits::Owned<'a> + 'static,
      <Results as ::traits::Pipelined>::Pipeline: FromTypelessPipeline
{
    pub fn send(self) -> RemotePromise<Results> {
        let RemotePromise {promise, pipeline, ..} = self.hook.send();
        let typed_promise = promise.map(|response| {
            Ok(Response {hook: response.hook,
                        marker: ::std::marker::PhantomData})
        });
        RemotePromise { promise: typed_promise,
                        pipeline: FromTypelessPipeline::new(pipeline)
                      }
    }
}

pub struct Params<T> {
    pub marker: ::std::marker::PhantomData<T>,
    pub hook: Box<ParamsHook>,
}

impl <T> Params <T> {
    pub fn new(hook: Box<ParamsHook>) -> Params<T> {
        Params { marker: ::std::marker::PhantomData, hook: hook }
    }
    pub fn get<'a>(&'a self) -> ::Result<<T as ::traits::Owned<'a>>::Reader>
        where T: ::traits::Owned<'a>
    {
        Ok(try!(try!(self.hook.get()).get_as()))
    }
}

pub struct Results<T> {
    pub marker: ::std::marker::PhantomData<T>,
    pub hook: Box<ResultsHook>,
}

impl <T> Results<T>
    where T: for<'a> ::traits::Owned<'a>
{
    pub fn new(hook: Box<ResultsHook>) -> Results<T> {
        Results { marker: ::std::marker::PhantomData, hook: hook }
    }

    pub fn get<'a>(&'a mut self) -> <T as ::traits::Owned<'a>>::Builder {
        self.hook.get().unwrap().get_as().unwrap()
    }

    pub fn set(&mut self, other: <T as ::traits::Owned>::Reader) -> ::Result<()>
    {
        self.hook.get().unwrap().set_as(other)
    }

}


pub trait FromTypelessPipeline {
    fn new (typeless: any_pointer::Pipeline) -> Self;
}

pub trait FromClientHook {
    fn new(Box<ClientHook>) -> Self;
}

pub struct Client {
    pub hook: Box<ClientHook>
}

impl Client {
    pub fn new(hook: Box<ClientHook>) -> Client {
        Client { hook : hook }
    }

    pub fn new_call<Params, Results>(&self,
                                     interface_id : u64,
                                     method_id : u16,
                                     size_hint : Option<::MessageSize>)
                                     -> Request<Params, Results> {
        let typeless = self.hook.new_call(interface_id, method_id, size_hint);
        Request { hook: typeless.hook, marker: ::std::marker::PhantomData }
    }
}

pub trait Server {
    fn dispatch_call(&mut self, interface_id: u64, method_id: u16,
                     params: Params<any_pointer::Owned>,
                     results: Results<any_pointer::Owned>)
                     -> Promise<(), ::Error>;
}

