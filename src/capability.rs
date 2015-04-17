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

// Things from capability.c++

use capnp::any_pointer;
use capnp::MessageSize;
use capnp::private::capability::{CallContextHook, Client, ClientHook, PipelineHook, ServerHook};
use capnp::capability::{CallContext, Request, ResultFuture, Server};
use capnp::traits::{FromPointerReader, FromPointerBuilder};
use capnp::{MessageReader, MessageBuilder};

use rpc_capnp::{message, return_};

pub struct LocalClient {
    object_channel : ::std::sync::mpsc::Sender<(u64, u16, Box<CallContextHook+Send>)>,
}

impl Clone for LocalClient {
    fn clone(&self) -> LocalClient {
        LocalClient { object_channel : self.object_channel.clone() }
    }
}

impl LocalClient {
    pub fn new(server : Box<Server+Send>) -> LocalClient {
        let (chan, port) = ::std::sync::mpsc::channel::<(u64, u16, Box<CallContextHook+Send>)>();
        ::std::thread::spawn(move || {
                let mut server = server;
                loop {
                    let (interface_id, method_id, context_hook) = match port.recv() {
                        Err(_) => break,
                        Ok(x) => x,
                    };

                    let context = CallContext { hook : context_hook, marker : ::std::marker::PhantomData };
                    server.dispatch_call(interface_id, method_id, context)
                }
            });

        LocalClient { object_channel : chan }
    }
}


impl ClientHook for LocalClient {
    fn copy(&self) -> Box<ClientHook+Send> {
        Box::new(LocalClient { object_channel : self.object_channel.clone() })
    }
    fn new_call(&self,
                _interface_id : u64,
                _method_id : u16,
                _size_hint : Option<MessageSize>)
                -> Request<any_pointer::Builder, any_pointer::Reader, any_pointer::Pipeline> {
        panic!()
    }
    fn call(&self, interface_id : u64, method_id : u16, context : Box<CallContextHook+Send>) {
        self.object_channel.send((interface_id, method_id, context)).unwrap();
    }

    // HACK
    fn get_descriptor(&self) -> Box<::std::any::Any> {
        Box::new(self.copy())
    }

}

impl ServerHook for LocalClient {
    fn new_client(_unused_self : Option<LocalClient>, server : Box<Server+Send>) -> Client {
        Client::new(Box::new(LocalClient::new(server)))
    }
}

pub trait InitRequest<'a, 'b, T> {
    fn init(&'b mut self) -> T;
}

impl <'a, 'b, Params : FromPointerBuilder<'a>, Results, Pipeline> InitRequest<'a, 'b, Params>
for Request<Params, Results, Pipeline> {

    // XXX we are bypassing lifetime tracking here.
    // TODO: maybe there's something clever we can do to get this to work.
    // We may have to wait for associated types or higher-kinded types.
    fn init(&'b mut self) -> Params {
        let tmp : &'a mut Box<::capnp::private::capability::RequestHook> = unsafe { ::std::mem::transmute(& mut self.hook)};
        let message : message::Builder = tmp.message::<'a>().get_root().unwrap();
        match message.which() {
            Ok(message::Call(Ok(call))) => {
                let params = call.init_params();
                params.get_content().init_as()
            }
            _ => panic!(),
        }
    }
}

pub trait WaitForContent<'a, T> {
    fn wait(&'a mut self) -> Result<T, String>;
}

impl <'a, Results : FromPointerReader<'a>, Pipeline> WaitForContent<'a, Results>
for ResultFuture<Results, Pipeline> {
    fn wait(&'a mut self) -> Result<Results, String> {
        // XXX should check that it's not already been received.
        self.answer_result = match self.answer_port.recv() {Ok(x) => Ok(x), Err(_) => Err(()) };
        match self.answer_result {
            Err(_) => Err("answer channel closed".to_string()),
            Ok(ref mut response_hook) => {
                let root : message::Reader = response_hook.get().get_as().unwrap();
                match root.which() {
                    Ok(message::Return(Ok(ret))) => {
                        match ret.which() {
                            Ok(return_::Results(Ok(res))) => {
                                Ok(res.get_content().get_as().unwrap())
                            }
                            Ok(return_::Exception(Ok(e))) => {
                                Err(e.get_reason().unwrap().to_string())
                            }
                            _ => panic!(),
                        }
                    }
                    _ => {panic!()}
                }
            }
        }
    }
}
