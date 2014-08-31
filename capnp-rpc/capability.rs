/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

// Things from capability.c++

use std;

use capnp::any_pointer;
use capnp::MessageSize;
use capnp::capability::{CallContext, CallContextHook, Client,
                        ClientHook, PipelineHook, Request, ResultFuture, Server, ServerHook};
use capnp::layout::{FromStructReader, FromStructBuilder, HasStructSize};
use capnp::{MessageReader, MessageBuilder};

use rpc_capnp::{message, return_};

pub struct LocalClient {
    object_channel : std::comm::Sender<(u64, u16, Box<CallContextHook+Send>)>,
}

impl Clone for LocalClient {
    fn clone(&self) -> LocalClient {
        LocalClient { object_channel : self.object_channel.clone() }
    }
}

impl LocalClient {
    pub fn new(server : Box<Server+Send>) -> LocalClient {
        let (chan, port) = std::comm::channel::<(u64, u16, Box<CallContextHook+Send>)>();
        std::task::spawn(proc () {
                let mut server = server;
                loop {
                    let (interface_id, method_id, context_hook) = match port.recv_opt() {
                        Err(_) => break,
                        Ok(x) => x,
                    };

                    let context = CallContext { hook : context_hook };
                    server.dispatch_call(interface_id, method_id, context)
                }
            });

        LocalClient { object_channel : chan }
    }
}


impl ClientHook for LocalClient {
    fn copy(&self) -> Box<ClientHook+Send> {
        (box LocalClient { object_channel : self.object_channel.clone() }) as Box<ClientHook+Send>
    }
    fn new_call(&self,
                _interface_id : u64,
                _method_id : u16,
                _size_hint : Option<MessageSize>)
                -> Request<any_pointer::Builder, any_pointer::Reader, any_pointer::Pipeline> {
        fail!()
    }
    fn call(&self, interface_id : u64, method_id : u16, context : Box<CallContextHook+Send>) {
        self.object_channel.send((interface_id, method_id, context));
    }

    // HACK
    fn get_descriptor(&self) -> Box<std::any::Any + 'static> {
        (box self.copy()) as Box<std::any::Any + 'static>
    }

}

impl ServerHook for LocalClient {
    fn new_client(_unused_self : Option<LocalClient>, server : Box<Server+Send>) -> Client {
        Client::new((box LocalClient::new(server)) as Box<ClientHook+Send>)
    }
}

pub trait InitRequest<'a, 'b, T> {
    fn init(&'b mut self) -> T;
}

impl <'a, 'b, Params : FromStructBuilder<'a> + HasStructSize, Results, Pipeline> InitRequest<'a, 'b, Params>
for Request<Params, Results, Pipeline> {

    // XXX we are bypassing lifetime tracking here.
    // TODO: maybe there's something clever we can do to get this to work.
    // We may have to wait for associated types or higher-kinded types.
    fn init(&'b mut self) -> Params {
        let tmp : &'a mut Box<::capnp::capability::RequestHook> = unsafe { ::std::mem::transmute(& mut self.hook)};
        let message : message::Builder = tmp.message::<'a>().get_root();
        match message.which() {
            Some(message::Call(call)) => {
                let params = call.init_params();
                params.get_content().init_as_struct()
            }
            _ => fail!(),
        }
    }
}

pub trait WaitForContent<'a, T> {
    fn wait(&'a mut self) -> Result<T, String>;
}

impl <'a, Results : FromStructReader<'a>, Pipeline> WaitForContent<'a, Results>
for ResultFuture<Results, Pipeline> {
    fn wait(&'a mut self) -> Result<Results, String> {
        // XXX should check that it's not already been received.
        self.answer_result = self.answer_port.recv_opt();
        match self.answer_result {
            Err(_) => Err("answer channel closed".to_string()),
            Ok(ref mut response_hook) => {
                let root : message::Reader = response_hook.get().get_as_struct();
                match root.which() {
                    Some(message::Return(ret)) => {
                        match ret.which() {
                            Some(return_::Results(res)) => {
                                Ok(res.get_content().get_as_struct())
                            }
                            Some(return_::Exception(e)) => {
                                Err(e.get_reason().to_string())
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
