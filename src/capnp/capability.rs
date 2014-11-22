/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use any_pointer;
use common::{MessageSize};
use traits::{FromPointerReader, FromPointerBuilder};
use message::{MallocMessageBuilder};
use std;
use std::vec::Vec;

pub struct ResultFuture<Results, Pipeline> {
    pub answer_port : std::comm::Receiver<Box<ResponseHook+Send>>,
    pub answer_result : Result<Box<ResponseHook+Send>, ()>,
    pub pipeline : Pipeline,
}

pub trait ResponseHook:Send {
    fn get<'a>(&'a mut self) -> any_pointer::Reader<'a>;
}

pub trait RequestHook {
    fn message<'a>(&'a mut self) -> &'a mut MallocMessageBuilder;
    fn send<'a>(self : Box<Self>) -> ResultFuture<any_pointer::Reader<'a>, any_pointer::Pipeline>;
}

pub struct Request<Params, Results, Pipeline> {
    pub hook : Box<RequestHook+Send>
}

impl <Params, Results, Pipeline > Request <Params, Results, Pipeline> {
    pub fn new(hook : Box<RequestHook+Send>) -> Request <Params, Results, Pipeline> {
        Request { hook : hook }
    }
}
impl <Params, Results, Pipeline : FromTypelessPipeline> Request <Params, Results, Pipeline> {
    pub fn send(self) -> ResultFuture<Results, Pipeline> {
        let ResultFuture {answer_port, answer_result, pipeline} = self.hook.send();
        ResultFuture { answer_port : answer_port, answer_result : answer_result,
                        pipeline : FromTypelessPipeline::new(pipeline) }
    }
}

pub trait FromClientHook {
    fn new(Box<ClientHook+Send>) -> Self;
}

pub trait ClientHook : Send {
    fn copy(&self) -> Box<ClientHook+Send>;
    fn new_call(&self,
                interface_id : u64,
                method_id : u16,
                size_hint : Option<MessageSize>)
                -> Request<any_pointer::Builder, any_pointer::Reader, any_pointer::Pipeline>;
    fn call(&self, interface_id : u64, method_id : u16, context : Box<CallContextHook+Send>);

    // HACK
    fn get_descriptor(&self) -> Box<std::any::Any + 'static>;
}

pub trait ServerHook {
    fn new_client(unused : Option<Self>, server : Box<Server+Send>) -> Client;
}

pub trait FromServer<T, U> {
    fn new(hook : Option<T>, server : Box<U>) -> Self;
}

pub struct Client {
    pub hook : Box<ClientHook+Send>
}

impl Client {
    pub fn new(hook : Box<ClientHook+Send>) -> Client {
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
    pub hook : Box<CallContextHook+Send>,
}

impl <Params, Results> CallContext<Params, Results> {
    pub fn fail(self) {self.hook.fail();}
    pub fn done(self) {self.hook.done();}
}

impl <'a, Params : FromPointerReader<'a>, Results : FromPointerBuilder<'a>>
CallContext<Params, Results> {
    // XXX this 'b lifetime should be 'a.
    pub fn get<'b>(&'b mut self) -> (Params, Results) {
        let tmp : &'a mut Box<CallContextHook+Send> = unsafe { ::std::mem::transmute(& mut self.hook)};
        let (any_params, any_results) = tmp.get();
        (any_params.get_as(), any_results.get_as())
    }
}

pub trait CallContextHook {
    fn get<'a>(&'a mut self) -> (any_pointer::Reader<'a>, any_pointer::Builder<'a>);
    fn fail(self : Box<Self>);
    fn done(self : Box<Self>);
}

pub trait Server {
    fn dispatch_call(&mut self, interface_id : u64, method_id : u16,
                     context : CallContext<any_pointer::Reader, any_pointer::Builder>);

}

// Where should this live?
pub fn internal_get_typed_context<Params, Results>(
    typeless : CallContext<any_pointer::Reader, any_pointer::Builder>)
    -> CallContext<Params, Results> {
    CallContext { hook : typeless.hook }
}


pub trait PipelineHook {
    fn copy(&self) -> Box<PipelineHook+Send>;
    fn get_pipelined_cap(&self, ops : Vec<PipelineOp>) -> Box<ClientHook+Send>;
}

#[deriving(Clone)]
pub enum PipelineOp {
    Noop,
    GetPointerField(u16),
}

pub trait FromTypelessPipeline {
    fn new (typeless : any_pointer::Pipeline) -> Self;
}
