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

use std;
use std::collections::hash_map::HashMap;
use capnp::{any_pointer, MessageBuilder, MallocMessageBuilder, ReaderOptions};
use capnp::private::capability::{ClientHook};
use capnp::capability::{FromClientHook, Server};
use rpc::{RpcConnectionState, RpcEvent, SturdyRefRestorer};
use capability::{LocalClient};

pub struct EzRpcClient {
    rpc_chan : std::sync::mpsc::Sender<RpcEvent>,
    tcp : ::std::net::TcpStream,
}

impl Drop for EzRpcClient {
    fn drop(&mut self) {
        self.rpc_chan.send(RpcEvent::Shutdown).is_ok();
        //self.tcp.close_read().is_ok();
    }
}

impl EzRpcClient {
    pub fn new(server_address : &str) -> std::io::Result<EzRpcClient> {
        let tcp = try!(::std::net::TcpStream::connect(server_address));

        let connection_state = RpcConnectionState::new();

        let chan = connection_state.run(::capnp::io::ReadInputStream::new(try!(tcp.try_clone())),
                                        ::capnp::io::WriteOutputStream::new(try!(tcp.try_clone())),
                                        (), *ReaderOptions::new().fail_fast(false));

        return Ok(EzRpcClient { rpc_chan : chan, tcp : tcp });
    }

    pub fn import_cap<T : FromClientHook>(&mut self, name : &str) -> T {
        let mut message = box MallocMessageBuilder::new_default();
        {
            let restore = message.init_root::<message::Builder>().init_bootstrap();
            restore.init_deprecated_object_id().set_as(name);
        }

        let (outgoing, answer_port, _question_port) = RpcEvent::new_outgoing(message);
        self.rpc_chan.send(RpcEvent::Outgoing(outgoing)).unwrap();

        let mut response_hook = answer_port.recv().unwrap();
        let message : message::Reader = response_hook.get().get_as();
        let client = match message.which() {
            Some(message::Return(ret)) => {
                match ret.which() {
                    Some(return_::Results(payload)) => {
                        payload.get_content().get_as_capability::<T>()
                    }
                    _ => { panic!() }
                }
            }
            _ => {panic!()}
        };

        return client;
    }
}

enum ExportEvent {
    Restore(String, std::sync::mpsc::Sender<Option<Box<ClientHook+Send>>>),
    Register(String, Box<Server+Send>),
}

struct ExportedCaps {
    objects : HashMap<String, Box<ClientHook+Send>>,
}

impl ExportedCaps {
    pub fn new() -> std::sync::mpsc::Sender<ExportEvent> {
        let (chan, port) = std::sync::mpsc::channel::<ExportEvent>();

        std::thread::spawn(move || {
                let mut vat = ExportedCaps { objects : HashMap::new() };

                loop {
                    match port.recv() {
                        Ok(ExportEvent::Register(name, server)) => {
                            vat.objects.insert(name, box LocalClient::new(server) as Box<ClientHook+Send>);
                        }
                        Ok(ExportEvent::Restore(name, return_chan)) => {
                            return_chan.send(Some(vat.objects[name].copy())).unwrap();
                        }
                        Err(_) => break,
                    }
                }
            });

        chan
    }
}

pub struct Restorer {
    sender : std::sync::mpsc::Sender<ExportEvent>,
}

impl Restorer {
    fn new(sender : std::sync::mpsc::Sender<ExportEvent>) -> Restorer {
        Restorer { sender : sender }
    }
}

impl SturdyRefRestorer for Restorer {
    fn restore(&self, obj_id : any_pointer::Reader) -> Option<Box<ClientHook+Send>> {
        let (tx, rx) = std::sync::mpsc::channel();
        self.sender.send(ExportEvent::Restore(obj_id.get_as::<::capnp::text::Reader>().to_string(), tx)).unwrap();
        return rx.recv().unwrap();
    }
}

pub struct EzRpcServer {
    sender : ::std::sync::mpsc::Sender<ExportEvent>,
    tcp_listener : ::std::net::TcpListener,
}

impl EzRpcServer {
    pub fn new(bind_address : &str) -> std::io::Result<EzRpcServer> {

        let tcp_listener = try!(::std::net::TcpListener::bind(bind_address));

        let sender = ExportedCaps::new();

        Ok(EzRpcServer { sender : sender, tcp_listener : tcp_listener  })
    }

    pub fn export_cap(&self, name : &str, server : Box<Server+Send>) {
        self.sender.send(ExportEvent::Register(name.to_string(), server)).unwrap()
    }

    pub fn serve<'a>(self) -> ::std::thread::JoinGuard<'a, ()> {
        std::thread::scoped(move || {
            let server = self;
            for stream_result in server.tcp_listener.incoming() {
                let tcp = stream_result.unwrap();
                let sender2 = server.sender.clone();
                std::thread::spawn(move || {
                    let connection_state = RpcConnectionState::new();
                    let _rpc_chan = connection_state.run(
                        ::capnp::io::ReadInputStream::new(tcp.try_clone().unwrap()),
                        ::capnp::io::WriteOutputStream::new(tcp),
                        Restorer::new(sender2),
                        *ReaderOptions::new().fail_fast(false));
                });
            }
        })
    }
}

