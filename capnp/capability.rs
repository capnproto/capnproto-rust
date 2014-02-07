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

pub struct RemotePromise<Results, Pipeline> {
    answer_port : std::comm::Port<~OwnedSpaceMessageReader>,
    answer_result : Option<~OwnedSpaceMessageReader>,
    pipeline : Pipeline,
}

pub trait RequestHook {
    fn message<'a>(&'a mut self) -> &'a mut MallocMessageBuilder;
    fn send(~self) -> RemotePromise<AnyPointer::Reader, AnyPointer::Pipeline>;
}

pub struct Request<Params, Results, Pipeline> {
    hook : ~RequestHook
}

impl <Params, Results, Pipeline > Request <Params, Results, Pipeline> {
    pub fn new(hook : ~RequestHook) -> Request <Params, Results, Pipeline> {
        Request { hook : hook }
    }
}
impl <Params, Results, Pipeline : FromTypelessPipeline> Request <Params, Results, Pipeline> {
    pub fn send(self) -> RemotePromise<Results, Pipeline> {
        let RemotePromise {answer_port, answer_result, pipeline} = self.hook.send();
        RemotePromise { answer_port : answer_port, answer_result : answer_result,
                        pipeline : FromTypelessPipeline::new(pipeline) }
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
                -> Request<AnyPointer::Builder, AnyPointer::Reader, AnyPointer::Pipeline>;

    // HACK
    fn get_descriptor(&self) -> ~std::any::Any;
}


pub struct Client {
    hook : ~ClientHook
}

impl Client {
    pub fn new(hook : ~ClientHook) -> Client {
        Client { hook : hook }
    }

    pub fn new_call<Params, Results, Pipeline>(&self,
                                               interface_id : u64,
                                               method_id : u16,
                                               size_hint : Option<MessageSize>)
                                               -> Request<Params, Results, Pipeline> {
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

pub trait Server {
    fn dispatch_call(&self, interface_id : u64, method_id : u16,
                     context : CallContext<AnyPointer::Reader, AnyPointer::Builder>);
}


pub trait PipelineHook {
    fn copy(&self) -> ~PipelineHook;
    fn get_pipelined_cap(&self, ops : ~[PipelineOp::Type]) -> ~ClientHook;
}

pub mod PipelineOp {

    #[deriving(Clone)]
    pub enum Type {
        Noop,
        GetPointerField(u16),
    }
}

pub trait FromTypelessPipeline {
    fn new (typeless : AnyPointer::Pipeline) -> Self;
}
