
#[crate_id="capnp-rpc"];
#[crate_type="bin"];

extern mod capnp;

extern mod extra;

pub mod async;

pub mod calculator_capnp;
pub mod rpc_capnp;
//pub mod rpc-twoparty_capnp;

pub mod rpc;

pub mod testing {
    use capnp::message::{MessageBuilder, MallocMessageBuilder, MessageReader};
    use capnp::serialize::{OwnedSpaceMessageReader};
    use calculator_capnp::Calculator;
    use rpc_capnp::{Message, Return};
    use rpc::{RpcEvent, OutgoingMessage, InitParams};
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
            exp.set_literal(1.2345e6);
        }
        let res = req.send();
        res.port.recv();
    }
}

pub fn main() {
    use std::io::process;

    let args = ~[std::os::args()[1].to_owned(), std::os::args()[2].to_owned()];

    let io = [process::CreatePipe(true, false), // stdin
              process::CreatePipe(false, true), // stdout
              process::InheritFd(2)];

    let config = process::ProcessConfig {
        program: "nc",
        args: args,
        env : None,
        cwd: None,
        io : io
    };
    let mut p = process::Process::new(config).unwrap();

    p.io.pop();
    let childStdOut = p.io.pop();
    let childStdIn = p.io.pop();

    let mut connection_state = rpc::RpcConnectionState::new();

    let chan = connection_state.run(childStdOut, childStdIn);

    testing::connect(chan);

    p.wait();
}
