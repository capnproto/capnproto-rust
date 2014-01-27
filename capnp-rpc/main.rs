
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
    use capnp;
    use capnp::message::{MessageBuilder, MallocMessageBuilder};
    use calculator_capnp::Calculator;
    use rpc_capnp::{Message};
    use std;

    pub fn connect<T : std::io::Writer>(out_stream : &mut T) {

        let mut message = MallocMessageBuilder::new_default();
        let restore = message.init_root::<Message::Builder>().init_restore();
        restore.set_question_id(0);
        restore.init_object_id().set_as_text("calculator");

        capnp::serialize::write_message(out_stream, &mut message);

        let mut message = MallocMessageBuilder::new_default();
        let call = message.init_root::<Message::Builder>().init_call();
        call.set_question_id(1);
        let promised_answer = call.init_target().init_promised_answer();
        promised_answer.set_question_id(0);
        call.set_interface_id(0x97983392df35cc36);
        call.set_method_id(0);
        let payload = call.init_params();
        let exp = payload.init_content().init_as_struct::<Calculator::Expression::Builder>();
        exp.set_literal(1.23456);

        capnp::serialize::write_message(out_stream, &mut message);
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
    let mut childStdIn = p.io.pop();

    let (port, chan) = std::comm::SharedChan::<rpc::RpcEvent>::new();

    let listener_chan = chan.clone();

    do spawn || {
        let mut r = childStdOut;

        loop {
            let message = box capnp::serialize::new_reader(
                &mut r,
                capnp::message::DEFAULT_READER_OPTIONS);

            listener_chan.send(rpc::IncomingMessage(message));
        }
    }

    do spawn || { rpc::run_loop(port); }

    testing::connect(&mut childStdIn);

    chan.send(rpc::Nothing);

    p.wait();
}
