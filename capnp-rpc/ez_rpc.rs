/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use rpc_capnp::{Message, Return};

use std;
use capnp::capability::{ClientHook, FromClientHook, ServerHook, Server, Client};
use capnp::message::{MessageBuilder, MallocMessageBuilder, MessageReader};
use rpc::{RpcConnectionState, RpcEvent, ShutdownEvent};
use rpc::{Vat, ObjectHandle};
use capability;

pub struct EzRpcClient {
    rpc_chan : std::comm::SharedChan<RpcEvent>,
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

        let tcp = if_ok!(tcp::TcpStream::connect(addr));

        let connection_state = RpcConnectionState::new();

        let chan = connection_state.run(tcp.clone(), tcp);

        return Ok(EzRpcClient { rpc_chan : chan });
    }

    pub fn import_cap<T : FromClientHook>(&mut self, name : &str) -> T {
        let mut message = ~MallocMessageBuilder::new_default();
        let restore = message.init_root::<Message::Builder>().init_restore();
        restore.init_object_id().set_as_text(name);

        let (event, answer_port, _question_port) = RpcEvent::new_outgoing(message);
        self.rpc_chan.send(event);

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
    fn new_client(&self, server : ~Server) -> Client {
        Client::new((~capability::LocalClient {
                    object :  ObjectHandle::new(server) }) as ~ClientHook)
    }
}



pub struct EzRpcServer {
    rpc_chan : std::comm::SharedChan<RpcEvent>,
    vat : Vat,
}

impl ServerHook for EzRpcServer {
    fn new_client(&self, server : ~Server) -> Client {
        Client::new((~capability::LocalClient {
                    object :  ObjectHandle::new(server) }) as ~ClientHook)
    }
}


pub struct EzRpcServerAcceptor {
    tcp_acceptor : std::io::net::tcp::TcpAcceptor,
}

impl std::io::Acceptor<EzRpcServer> for EzRpcServerAcceptor {
    fn accept(&mut self) -> std::io::IoResult<EzRpcServer> {
        let tcp = if_ok!(self.tcp_acceptor.accept());

        let connection_state = RpcConnectionState::new();

        let chan = connection_state.run(tcp.clone(), tcp);

        return Ok(EzRpcServer { rpc_chan : chan, vat : Vat::new() });

    }
}

impl EzRpcServer {
    pub fn acceptor(bind_address : &str) -> std::io::IoResult<EzRpcServerAcceptor> {
        use std::io::net::{ip, tcp};
        use std::io::Listener;

        let addr : ip::SocketAddr = FromStr::from_str(bind_address).expect("bad bind address");

        let tcp_listener = if_ok!(tcp::TcpListener::bind(addr));

        let tcp_acceptor = if_ok!(tcp_listener.listen());

        Ok(EzRpcServerAcceptor { tcp_acceptor : tcp_acceptor })
    }

}
