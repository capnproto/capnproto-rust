/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

// Things from capability.c++

use std;

use capnp::any::{AnyPointer};
use capnp::common::{MessageSize};
use capnp::capability::{ClientHook, Request, RemotePromise};
use capnp::layout::{FromStructReader, FromStructBuilder, HasStructSize};
use capnp::message::{MessageReader, MessageBuilder};
use rpc::{ObjectHandle};

use rpc_capnp::{Message, Return};

pub struct LocalClient {
    object : ObjectHandle,
}

impl Clone for LocalClient {
    fn clone(&self) -> LocalClient {
        LocalClient { object : self.object.clone() }
    }

}

impl ClientHook for LocalClient {
    fn copy(&self) -> ~ClientHook {
        (~LocalClient { object : self.object.clone() }) as ~ClientHook
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
        (~self.object.clone()) as ~std::any::Any
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
