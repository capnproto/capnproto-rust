/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use rpc_capnp::{Message, Return};

use std;
use std::io::Acceptor;
use collections::hashmap::HashMap;
use capnp::{AnyPointer, MessageBuilder, MallocMessageBuilder};
use capnp::capability::{ClientHook, FromClientHook, Server};
use rpc::{Outgoing, RpcConnectionState, RpcEvent, ShutdownEvent, SturdyRefRestorer};
use capability::{LocalClient};

pub struct EzRpcClient {
    rpc_chan : std::comm::Sender<RpcEvent>,
    tcp : std::io::net::tcp::TcpStream,
}

impl Drop for EzRpcClient {
    fn drop(&mut self) {
        self.rpc_chan.send_opt(ShutdownEvent).is_ok();
        self.tcp.close_read().is_ok();
    }
}

impl EzRpcClient {
    pub fn new(server_address : &str) -> std::io::IoResult<EzRpcClient> {
        use std::io::net::{ip, tcp};

        let addr : ip::SocketAddr = std::from_str::FromStr::from_str(server_address).expect("bad server address");

        let tcp = try!(tcp::TcpStream::connect(format!("{}", addr.ip),  addr.port));

        let connection_state = RpcConnectionState::new();

        let chan = connection_state.run(tcp.clone(), tcp.clone(), ());

        return Ok(EzRpcClient { rpc_chan : chan, tcp : tcp });
    }

    pub fn import_cap<T : FromClientHook>(&mut self, name : &str) -> T {
        let mut message = box MallocMessageBuilder::new_default();
        let restore = message.init_root::<Message::Builder>().init_restore();
        restore.init_object_id().set_as_text(name);

        let (outgoing, answer_port, _question_port) = RpcEvent::new_outgoing(message);
        self.rpc_chan.send(Outgoing(outgoing));

        let mut response_hook = answer_port.recv();
        let message : Message::Reader = response_hook.get().get_as_struct();
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

enum ExportEvent {
    ExportEventRestore(~str, std::comm::Sender<Option<Box<ClientHook:Send>>>),
    ExportEventRegister(~str, Box<Server:Send>),
}

struct ExportedCaps {
    objects : HashMap<~str, Box<ClientHook:Send>>,
}

impl ExportedCaps {
    pub fn new() -> std::comm::Sender<ExportEvent> {
        let (chan, port) = std::comm::channel::<ExportEvent>();

        std::task::spawn(proc() {
                let mut vat = ExportedCaps { objects : HashMap::new() };

                loop {
                    match port.recv_opt() {
                        Ok(ExportEventRegister(name, server)) => {
                            vat.objects.insert(name, box LocalClient::new(server) as Box<ClientHook:Send>);
                        }
                        Ok(ExportEventRestore(name, return_chan)) => {
                            return_chan.send(Some((*vat.objects.get(&name)).copy()));
                        }
                        Err(_) => break,
                    }
                }
            });

        chan
    }
}

pub struct Restorer {
    sender : std::comm::Sender<ExportEvent>,
}

impl Restorer {
    fn new(sender : std::comm::Sender<ExportEvent>) -> Restorer {
        Restorer { sender : sender }
    }
}

impl SturdyRefRestorer for Restorer {
    fn restore(&self, obj_id : AnyPointer::Reader) -> Option<Box<ClientHook:Send>> {
        let (tx, rx) = std::comm::channel();
        self.sender.send(ExportEventRestore(obj_id.get_as_text().to_owned(), tx));
        return rx.recv();
    }
}

pub struct EzRpcServer {
    sender : std::comm::Sender<ExportEvent>,
    tcp_acceptor : std::io::net::tcp::TcpAcceptor,
}

impl EzRpcServer {
    pub fn new(bind_address : &str) -> std::io::IoResult<EzRpcServer> {
        use std::io::net::{ip, tcp};
        use std::io::Listener;

        let addr : ip::SocketAddr = std::from_str::FromStr::from_str(bind_address).expect("bad bind address");

        let tcp_listener = try!(tcp::TcpListener::bind(format!("{}", addr.ip), addr.port));

        let tcp_acceptor = try!(tcp_listener.listen());

        let sender = ExportedCaps::new();

        Ok(EzRpcServer { sender : sender, tcp_acceptor : tcp_acceptor  })
    }

    pub fn export_cap(&self, name : &str, server : Box<Server:Send>) {
        self.sender.send(ExportEventRegister(name.to_owned(), server))
    }

    pub fn serve(self) {
        std::task::spawn(proc() {
            let mut server = self;
            for res in server.incoming() {
                match res {
                    Ok(()) => {}
                    Err(e) => {
                        println!("error: {}", e)
                    }
                }
            }

        });
    }
}

impl std::io::Acceptor<()> for EzRpcServer {
    fn accept(&mut self) -> std::io::IoResult<()> {

        let sender2 = self.sender.clone();
        let tcp = try!(self.tcp_acceptor.accept());
        std::task::spawn(proc() {
            let connection_state = RpcConnectionState::new();
            let _rpc_chan = connection_state.run(tcp.clone(), tcp, Restorer::new(sender2));
        });
        Ok(())
    }
}
