/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


#[crate_id="calculator"];
#[crate_type="bin"];

extern mod capnp;
extern mod extra;
extern mod capnp_rpc = "capnp-rpc";

pub mod calculator_capnp;

//pub mod rpc-twoparty_capnp;

pub mod testing {
    use capnp::message::{MessageBuilder, MallocMessageBuilder, MessageReader};
    use capnp::serialize::{OwnedSpaceMessageReader};
    use calculator_capnp::Calculator;
    use capnp_rpc::rpc_capnp::{Message, Return};
    use capnp_rpc::rpc::{RpcEvent, OutgoingMessage, InitParams, WaitForContent};
    use std;

    pub fn connect(rpc_chan : std::comm::SharedChan<RpcEvent>) {

        let mut message = ~MallocMessageBuilder::new_default();
        let restore = message.init_root::<Message::Builder>().init_restore();
        restore.set_question_id(0);
        restore.init_object_id().set_as_text("calculator");

        let (port, chan) = std::comm::Chan::<~OwnedSpaceMessageReader>::new();

        rpc_chan.send(OutgoingMessage(message, chan));

        let reader = port.recv();
        let message = reader.get_root::<Message::Reader>();
        let client = match message.which() {
            Some(Message::Return(ret)) => {
                match ret.which() {
                    Some(Return::Results(payload)) => {
                        payload.get_content().get_as_capability::<Calculator::Client>()
                    }
                    _ => { fail!() }
                }
            }
            _ => {fail!()}
        };

        let mut req = client.evaluate_request();
        {
            let params = req.init_params();
            let exp = params.init_expression();
            exp.set_literal(123.45);
        }
        let mut res = req.send();
        let value = {
            let results = res.wait();
            results.get_value()
        };

        let mut result = value.read_request().send();
        println!("the value is: {}", result.wait().get_value());

    }
}

pub fn main() {
    use std::io::process;

    let args = std::os::args();

    if args.len() != 3 {
        println!("usage: {} <ip address> <port number>", args[0]);
        return;
    }

    let child_args = ~[args[1].to_owned(), args[2].to_owned()];

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

    let connection_state = capnp_rpc::rpc::RpcConnectionState::new();

    let chan = connection_state.run(childStdOut, childStdIn);

    testing::connect(chan);

    p.wait();
}
