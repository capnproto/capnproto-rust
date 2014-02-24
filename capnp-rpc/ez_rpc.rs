/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use rpc_capnp::{Message, Return};

use std;
use std::io::Acceptor;
use capnp::capability::{ClientHook, FromClientHook, ServerHook, Server, Client};
use capnp::message::{MessageBuilder, MallocMessageBuilder, MessageReader};
use rpc::{Outgoing, RpcConnectionState, RpcEvent, ShutdownEvent, VatEvent, VatEventRegister};
use rpc::{Vat};
use capability;

pub struct EzRpcClient {
    rpc_chan : std::comm::Chan<RpcEvent>,
}

impl Drop for EzRpcClient {
    fn drop(&mut self) {
        self.rpc_chan.send(ShutdownEvent);
    }
}

impl EzRpcClient {
    pub fn new(server_address : &str) -> std::io::IoResult<EzRpcClient> {
        use std::io::net::{ip, tcp};

        let addr : ip::SocketAddr = FromStr::from_str(server_address).expect("bad server address");

        let tcp = try!(tcp::TcpStream::connect(addr));

        let vat_chan = Vat::new();
        let connection_state = RpcConnectionState::new();

        let chan = connection_state.run(tcp.clone(), tcp, vat_chan);

        return Ok(EzRpcClient { rpc_chan : chan });
    }

    pub fn import_cap<T : FromClientHook>(&mut self, name : &str) -> T {
        let mut message = ~MallocMessageBuilder::new_default();
        let restore = message.init_root::<Message::Builder>().init_restore();
        restore.init_object_id().set_as_text(name);

        let (outgoing, answer_port, _question_port) = RpcEvent::new_outgoing(message);
        self.rpc_chan.send(Outgoing(outgoing));

        let reader = answer_port.recv();
        let message = reader.get_root::<Message::Reader>();
        let client = match message.which() {
            Some(Message::Return(ret)) => {
                match ret.which() {
                    Some(Return::Results(payload)) => {
                        payload.get_content().get_as_capability::<T>()
                    }
                    _ => { fail!() }
                }
            }
            _ => {fail!()}
        };

        return client;
    }
}

impl ServerHook for EzRpcClient {
    fn new_client(_unused_self : Option<EzRpcClient>, server : ~Server) -> Client {
        Client::new((~capability::LocalClient::new(server) ) as ~ClientHook)
    }
}



pub struct EzRpcServer {
    vat_chan : std::comm::Chan<VatEvent>,
}

impl ServerHook for EzRpcServer {
    fn new_client(_unused_self : Option<EzRpcServer>, server : ~Server) -> Client {
        Client::new((~capability::LocalClient::new(server)) as ~ClientHook)
    }
}

impl EzRpcServer {
    pub fn new(bind_address : &str) -> std::io::IoResult<EzRpcServer> {
        use std::io::net::{ip, tcp};
        use std::io::Listener;

        let addr : ip::SocketAddr = FromStr::from_str(bind_address).expect("bad bind address");

        let tcp_listener = try!(tcp::TcpListener::bind(addr));

        let tcp_acceptor = try!(tcp_listener.listen());

        let vat_chan = Vat::new();

        let vat_chan2 = vat_chan.clone();
        std::task::spawn(proc() {
                let mut tcp_acceptor = tcp_acceptor;
                for tcp_result in tcp_acceptor.incoming() {
                    let tcp = tcp_result.unwrap();
                    let connection_state = RpcConnectionState::new();
                    let _rpc_chan = connection_state.run(tcp.clone(), tcp, vat_chan2.clone());
                }
            });

        Ok(EzRpcServer { vat_chan : vat_chan })
    }

    pub fn export_cap(&self, name : &str, server : ~Server) {
        self.vat_chan.send(VatEventRegister(name.to_owned(), server))
    }
}
