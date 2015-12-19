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

use any_pointer;
use MessageSize;
use capability::{Params, Request, RemotePromise, Results};

pub trait ResponseHook {
    fn get<'a>(&'a self) -> ::Result<any_pointer::Reader<'a>>;
}

pub trait RequestHook {
    fn init<'a>(&'a mut self) -> any_pointer::Builder<'a>;
    fn send<'a>(self : Box<Self>) -> RemotePromise<any_pointer::Owned>;
}

pub trait ClientHook {
    fn add_ref(&self) -> Box<ClientHook>;
    fn new_call(&self,
                interface_id: u64,
                method_id: u16,
                size_hint: Option<MessageSize>)
                -> Request<any_pointer::Owned, any_pointer::Owned>;

    fn call(&self, interface_id: u64, method_id: u16,
            params: Box<ParamsHook>, results: Box<ResultsHook>);

    fn get_brand(&self) -> usize;
    fn write_target(&self, target: any_pointer::Builder) -> Option<Box<ClientHook>>;

    // HACK
    fn get_descriptor(&self) -> Box<::std::any::Any>;
}

impl Clone for Box<ClientHook> {
    fn clone(&self) -> Box<ClientHook> {
        self.add_ref()
    }
}

#[cfg(feature = "rpc")]
pub trait ServerHook: 'static {
    fn new_client(server: Box<::capability::Server>) -> ::capability::Client;
}

pub trait ResultsHook {
    fn get<'a>(&'a mut self) -> any_pointer::Builder<'a>;
    fn fail(self: Box<Self>, message: String);
    fn unimplemented(self: Box<Self>);
    fn disconnected(self: Box<Self>);
    fn overloaded(self: Box<Self>);
}

pub trait ParamsHook {
    fn get<'a>(&'a self) -> any_pointer::Reader<'a>;
}

// Where should this live?
pub fn internal_get_typed_params<T>(typeless: Params<any_pointer::Owned>) -> Params<T> {
    Params { hook: typeless.hook, marker: ::std::marker::PhantomData }
}

pub fn internal_get_typed_results<T>(typeless: Results<any_pointer::Owned>) -> Results<T> {
    Results { hook: typeless.hook, marker: ::std::marker::PhantomData }
}

pub trait PipelineHook {
    fn add_ref(&self) -> Box<PipelineHook>;
    fn get_pipelined_cap(&self, ops: &[PipelineOp]) -> Box<ClientHook>;

    /// Version of get_pipelined_cap() passing the array by move. May avoid a copy in some cases.
    /// Default implementation just calls the other version.
    fn get_pipelined_cap_move(&self, ops: Vec<PipelineOp>) -> Box<ClientHook> {
        self.get_pipelined_cap(&ops)
    }
}

#[derive(Clone, Copy)]
pub enum PipelineOp {
    Noop,
    GetPointerField(u16),
}

