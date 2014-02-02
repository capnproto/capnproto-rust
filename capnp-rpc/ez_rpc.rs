/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


use std;
use capnp::capability::{FromClientHook};
use capnp::message::{MessageBuilder, MallocMessageBuilder, MessageReader};
use capnp::serialize::{OwnedSpaceMessageReader};
use rpc_capnp::{Message, Return};
use rpc::{RpcConnectionState, RpcEvent, OutgoingMessage};

pub struct EzRpcClient {
    chan : std::comm::SharedChan<RpcEvent>,
    netcat : std::io::process::Process,
}

impl EzRpcClient {
    pub fn new(server_address : &str) -> EzRpcClient {
        use std::io::process;
        use std::io::net::ip::SocketAddr;

        let addr : SocketAddr = FromStr::from_str(server_address).expect("bad server address");

        let child_args = ~[addr.ip.to_str(), addr.port.to_str()];

        let io = [process::CreatePipe(true, false), // stdin
                  process::CreatePipe(false, true), // stdout
                  process::InheritFd(2)];

        let config = process::ProcessConfig {
            program: "nc",
            args: child_args,
            env : None,
            cwd: None,
            io : io
        };
        let mut p = process::Process::new(config).unwrap();

        p.io.pop();
        let childStdOut = p.io.pop();
        let childStdIn = p.io.pop();

        let connection_state = RpcConnectionState::new();

        let chan = connection_state.run(childStdOut, childStdIn);

        return EzRpcClient { chan : chan, netcat : p };
    }

    pub fn import_cap<T : FromClientHook>(&mut self, name : &str) -> T {
        let mut message = ~MallocMessageBuilder::new_default();
        let restore = message.init_root::<Message::Builder>().init_restore();
        restore.init_object_id().set_as_text(name);

        let (port, chan) = std::comm::Chan::<~OwnedSpaceMessageReader>::new();

        self.chan.send(OutgoingMessage(message, chan));

        let reader = port.recv();
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

pub struct EzRpcServer {
    chan : std::comm::SharedChan<RpcEvent>,
}
