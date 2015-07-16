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

use rpc_capnp::{message, return_};

use capnp::private::capability::{ClientHook};
use capnp::capability::{FromClientHook, Server};
use rpc::{RpcConnectionState, RpcEvent};
use capability::{LocalClient};

pub struct EzRpcClient {
    rpc_chan : ::std::sync::mpsc::Sender<RpcEvent>,
    tcp : ::std::net::TcpStream,
}

impl Drop for EzRpcClient {
    fn drop(&mut self) {
        self.rpc_chan.send(RpcEvent::Shutdown).is_ok();
        //self.tcp.close_read().is_ok();
    }
}

struct EmptyCap;

impl Server for EmptyCap {
    fn dispatch_call(&mut self, _interface_id : u64, _method_id : u16,
                     context : ::capnp::capability::CallContext<::capnp::any_pointer::Reader,
                                                                ::capnp::any_pointer::Builder>) {
        context.fail("Attempted to call a method on an empty capability.".to_string());
    }
}

impl EzRpcClient {
    pub fn new<A: ::std::net::ToSocketAddrs>(server_address : A) -> ::std::io::Result<EzRpcClient> {
        let tcp = try!(::std::net::TcpStream::connect(server_address));

        let connection_state = RpcConnectionState::new();

        let empty_cap = Box::new(EmptyCap);
        let bootstrap = Box::new(LocalClient::new(empty_cap));

        let chan = connection_state.run(try!(tcp.try_clone()),
                                        try!(tcp.try_clone()),
                                        bootstrap,
                                        ::capnp::message::ReaderOptions::new());

        return Ok(EzRpcClient { rpc_chan : chan, tcp : tcp });
    }

    pub fn get_main<T : FromClientHook>(&mut self) -> T {
        let mut message = Box::new(::capnp::message::Builder::new_default());
        {
            message.init_root::<message::Builder>().init_bootstrap();
        }

        let (outgoing, answer_port, _question_port) = RpcEvent::new_outgoing(message);
        self.rpc_chan.send(RpcEvent::Outgoing(outgoing)).unwrap();

        let mut response_hook = answer_port.recv().unwrap();
        let message : message::Reader = response_hook.get().get_as().unwrap();
        let client = match message.which() {
            Ok(message::Return(Ok(ret))) => {
                match ret.which() {
                    Ok(return_::Results(Ok(payload))) => {
                        payload.get_content().get_as_capability::<T>().unwrap()
                    }
                    _ => { panic!() }
                }
            }
            _ => {panic!()}
        };

        return client;
    }
}

pub struct EzRpcServer {
     tcp_listener : ::std::net::TcpListener,
}

impl EzRpcServer {
    pub fn new<A: ::std::net::ToSocketAddrs>(bind_address : A) -> ::std::io::Result<EzRpcServer> {
        let tcp_listener = try!(::std::net::TcpListener::bind(bind_address));
        Ok(EzRpcServer { tcp_listener : tcp_listener  })
    }

    pub fn serve<'a>(self, bootstrap_interface : Box<Server + Send>) {
        let server = self;
        let bootstrap_interface = Box::new(LocalClient::new(bootstrap_interface));
        for stream_result in server.tcp_listener.incoming() {
            let bootstrap_interface = bootstrap_interface.copy();
            let tcp = stream_result.unwrap();
            ::std::thread::spawn(move || {
                let connection_state = RpcConnectionState::new();
                let _rpc_chan = connection_state.run(
                    tcp.try_clone().unwrap(),
                    tcp,
                    bootstrap_interface,
                    ::capnp::message::ReaderOptions::new());
            });
        }
    }
}

