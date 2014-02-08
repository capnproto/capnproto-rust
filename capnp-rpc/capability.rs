/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

// Things from capability.c++

use std;

use capnp::any::{AnyPointer};
use capnp::common::{MessageSize};
use capnp::capability::{CallContext, ClientHook, Request, RemotePromise};
use capnp::layout::{FromStructReader, FromStructBuilder, HasStructSize};
use capnp::message::{MessageReader, MessageBuilder, MallocMessageBuilder};
use rpc::{ExportId, SenderHosted};

use rpc_capnp::{Message, Return};

pub struct LocalClient {
    export_id : ExportId,
}

impl ClientHook for LocalClient {
    fn copy(&self) -> ~ClientHook {
        (~LocalClient { export_id : self.export_id }) as ~ClientHook
    }
    fn new_call(&self,
                _interface_id : u64,
                _method_id : u16,
                _size_hint : Option<MessageSize>)
                -> Request<AnyPointer::Builder, AnyPointer::Reader, AnyPointer::Pipeline> {
        fail!()
    }

    // HACK
    fn get_descriptor(&self) -> ~std::any::Any {
        (~SenderHosted(self.export_id)) as ~std::any::Any
    }

}

pub trait InitRequest<'a, T> {
    fn init(&'a mut self) -> T;
}

impl <'a, Params : FromStructBuilder<'a> + HasStructSize, Results, Pipeline> InitRequest<'a, Params>
for Request<Params, Results, Pipeline> {
    fn init(&'a mut self) -> Params {
        let message : Message::Builder = self.hook.message().get_root();
        match message.which() {
            Some(Message::Which::Call(call)) => {
                let params = call.init_params();
                params.get_content().init_as_struct()
            }
            _ => fail!(),
        }
    }
}

pub trait GetParams<'a, T> {
    fn get_params(&'a self) -> T;
}

impl <'a, Params : FromStructReader<'a>, Results> GetParams<'a, Params>
for CallContext<Params, Results> {
    fn get_params(&'a self) -> Params {
        let message : Message::Reader = self.hook.params_message().get_root();
        match message.which() {
            Some(Message::Call(call)) => {
                let params = call.get_params();
                params.get_content().get_as_struct()
            }
            _ => fail!(),
        }
    }
}

pub trait GetResults<'a, T> {
    fn get_results(&'a mut self) -> T;
}

impl <'a, Params, Results : FromStructBuilder<'a> + HasStructSize> GetResults<'a, Results>
for CallContext<Params, Results> {
    fn get_results(&'a mut self) -> Results {
        let message : Message::Builder = self.hook.results_message().get_root();
        match message.which() {
            Some(Message::Which::Call(call)) => {
                call.get_params().get_content().get_as_struct()
            }
            _ => fail!(),
        }
    }
}


pub trait WaitForContent<'a, T> {
    fn wait(&'a mut self) -> T;
}

impl <'a, Results : FromStructReader<'a>, Pipeline> WaitForContent<'a, Results>
for RemotePromise<Results, Pipeline> {
    fn wait(&'a mut self) -> Results {
        // XXX should check that it's not already been received.
        let message = self.answer_port.recv();
        self.answer_result = Some(message);
        match self.answer_result {
            None => unreachable!(),
            Some(ref message) => {
                let root : Message::Reader = message.get_root();
                match root.which() {
                    Some(Message::Return(ret)) => {
                        match ret.which() {
                            Some(Return::Results(res)) => {
                                res.get_content().get_as_struct()
                            }
                            _ => fail!(),
                        }
                    }
                    _ => {fail!()}
                }
            }
        }
    }
}
