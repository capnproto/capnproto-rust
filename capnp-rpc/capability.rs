/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

// Things from capability.c++

use std;

use capnp::any::{AnyPointer};
use capnp::common::{MessageSize};
use capnp::capability::{CallContext, CallContextHook, ClientHook, PipelineHook, Request, ResultFuture, Server};
use capnp::layout::{FromStructReader, FromStructBuilder, HasStructSize};
use capnp::message::{MessageReader, MessageBuilder};

use rpc_capnp::{Message, Return};

pub struct LocalClient {
    object_channel : std::comm::Chan<(u64, u16, ~CallContextHook)>,
}

impl Clone for LocalClient {
    fn clone(&self) -> LocalClient {
        LocalClient { object_channel : self.object_channel.clone() }
    }
}

impl LocalClient {
    pub fn new(server : ~Server) -> LocalClient {
        let (port, chan) = std::comm::Chan::<(u64, u16, ~CallContextHook)>::new();
        std::task::spawn(proc () {
                let mut server = server;
                loop {
                    let (interface_id, method_id, context_hook) = match port.recv_opt() {
                        None => break,
                        Some(x) => x,
                    };

                    let context = CallContext { hook : context_hook };
                    server.try_dispatch_call(interface_id, method_id, context)
                }
            });

        LocalClient { object_channel : chan }
    }
}


impl ClientHook for LocalClient {
    fn copy(&self) -> ~ClientHook {
        (~LocalClient { object_channel : self.object_channel.clone() }) as ~ClientHook
    }
    fn new_call(&self,
                _interface_id : u64,
                _method_id : u16,
                _size_hint : Option<MessageSize>)
                -> Request<AnyPointer::Builder, AnyPointer::Reader, AnyPointer::Pipeline> {
        fail!()
    }
    fn call(&self, interface_id : u64, method_id : u16, context : ~CallContextHook) {
        self.object_channel.send((interface_id, method_id, context));
    }

    // HACK
    fn get_descriptor(&self) -> ~std::any::Any {
        (~self.copy()) as ~std::any::Any
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
            Some(Message::Call(call)) => {
                let params = call.init_params();
                params.get_content().init_as_struct()
            }
            _ => fail!(),
        }
    }
}

pub trait WaitForContent<'a, T> {
    fn wait(&'a mut self) -> Result<T, ~str>;
}

impl <'a, Results : FromStructReader<'a>, Pipeline> WaitForContent<'a, Results>
for ResultFuture<Results, Pipeline> {
    fn wait(&'a mut self) -> Result<Results, ~str> {
        // XXX should check that it's not already been received.
        self.answer_result = self.answer_port.recv_opt();
        match self.answer_result {
            None => Err(~"answer channel closed"),
            Some(ref message) => {
                let root : Message::Reader = message.get_root();
                match root.which() {
                    Some(Message::Return(ret)) => {
                        match ret.which() {
                            Some(Return::Results(res)) => {
                                Ok(res.get_content().get_as_struct())
                            }
                            Some(Return::Exception(e)) => {
                                Err(e.get_reason().to_owned())
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
