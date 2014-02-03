/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use any::{AnyPointer};
use common::{MessageSize};
use message::{MallocMessageBuilder};
use serialize::{OwnedSpaceMessageReader};
use std;

pub struct RemotePromise<T> {
    port : std::comm::Port<~OwnedSpaceMessageReader>,
    result : Option<~OwnedSpaceMessageReader>,
}

pub trait RequestHook {
    fn message<'a>(&'a mut self) -> &'a mut MallocMessageBuilder;
    fn send(~self) -> RemotePromise<AnyPointer::Reader>;
}

pub struct Request<Params, Results> {
    hook : ~RequestHook
}

impl <Params, Results> Request <Params, Results> {
    pub fn new(hook : ~RequestHook) -> Request <Params, Results> {
        Request { hook : hook }
    }

    pub fn send(self) -> RemotePromise<Results> {
        let promise = self.hook.send();
        RemotePromise { port : promise.port, result : None }
    }
}

pub trait FromClientHook {
    fn new(~ClientHook) -> Self;
}

pub trait ClientHook {
    fn copy(&self) -> ~ClientHook;
    fn new_call(&self,
                interface_id : u64,
                method_id : u16,
                size_hint : Option<MessageSize>)
                -> Request<AnyPointer::Builder, AnyPointer::Reader>;
}


pub struct Client {
    hook : ~ClientHook
}

impl Client {
    pub fn new(hook : ~ClientHook) -> Client {
        Client { hook : hook }
    }

    pub fn new_call<Params, Results>(&self,
                                     interface_id : u64,
                                     method_id : u16,
                                     size_hint : Option<MessageSize>)
                                     -> Request<Params, Results> {
        let typeless = self.hook.new_call(interface_id, method_id, size_hint);
        Request { hook : typeless.hook }
    }
}

pub struct CallContext<Params, Results> {
    hook : ~CallContextHook,
}

pub trait CallContextHook {
    fn get_params<'a>(&'a self) -> AnyPointer::Reader<'a>;
}

pub trait PipelineHook {
    fn copy(&self) -> ~PipelineHook;
    fn get_pipelined_cap(&self, ops : &[PipelineOp::Type]) -> ~ClientHook;
}

pub mod PipelineOp {
    pub enum Type {
        Noop,
        GetPointerField(u16),
    }
}
