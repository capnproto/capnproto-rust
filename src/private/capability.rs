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
use capability::{CallContext, Request, ResultFuture, Server};

pub trait ResponseHook:Send + ::std::any::Any {
    fn get<'a>(&'a mut self) -> any_pointer::Reader<'a>;
}

pub trait RequestHook {
    fn message<'a>(&'a mut self) -> &'a mut ::message::Builder<::message::HeapAllocator>;
    fn send<'a>(self : Box<Self>) -> ResultFuture<any_pointer::Marker>;
}

pub trait ClientHook : Send + ::std::any::Any {
    fn copy(&self) -> Box<ClientHook+Send>;
    fn new_call(&self,
                interface_id : u64,
                method_id : u16,
                size_hint : Option<MessageSize>)
                -> Request<any_pointer::Marker, any_pointer::Marker>;
    fn call(&self, interface_id : u64, method_id : u16, context : Box<CallContextHook+Send>);

    // HACK
    fn get_descriptor(&self) -> Box<::std::any::Any>;
}

pub trait ServerHook : 'static {
    fn new_client(server : Box<Server+Send>) -> Client;
}

pub struct Client {
    pub hook : Box<ClientHook+Send>
}

impl Client {
    pub fn new(hook : Box<ClientHook+Send>) -> Client {
        Client { hook : hook }
    }

    pub fn new_call<Params, Results>(&self,
                                     interface_id : u64,
                                     method_id : u16,
                                     size_hint : Option<MessageSize>)
                                     -> Request<Params, Results> {
        let typeless = self.hook.new_call(interface_id, method_id, size_hint);
        Request { hook : typeless.hook, marker : ::std::marker::PhantomData }
    }
}

pub trait CallContextHook {
    fn get<'a>(&'a mut self) -> (any_pointer::Reader<'a>, any_pointer::Builder<'a>);
    fn fail(self : Box<Self>, message : String);
    fn done(self : Box<Self>);
}

// Where should this live?
pub fn internal_get_typed_context<Params, Results>(
    typeless : CallContext<any_pointer::Reader, any_pointer::Builder>)
    -> CallContext<Params, Results> {
    CallContext { hook : typeless.hook, marker : ::std::marker::PhantomData }
}


pub trait PipelineHook {
    fn copy(&self) -> Box<PipelineHook+Send>;
    fn get_pipelined_cap(&self, ops : Vec<PipelineOp>) -> Box<ClientHook+Send>;
}

#[derive(Clone, Copy)]
pub enum PipelineOp {
    Noop,
    GetPointerField(u16),
}

