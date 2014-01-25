
#[crate_id="capnp-rpc"];
#[crate_type="bin"];

extern mod capnp;

extern mod extra;

pub mod async;

pub mod calculator_capnp;
pub mod rpc_capnp;
//pub mod rpc-twoparty_capnp;


pub mod testing {
    use capnp;
    use calculator_capnp::Calculator;
    use rpc_capnp::{Message, Return, CapDescriptor};
    use std;

    pub fn connect<T : std::io::Writer>(out_stream : &mut T) {

        capnp::message::MessageBuilder::new_default(|message| {
                let restore = message.init_root::<Message::Builder>().init_restore();
                restore.set_question_id(0);
                restore.init_object_id().set_as_text("calculator");

                capnp::serialize::write_message(out_stream, message);
            });

        capnp::message::MessageBuilder::new_default(|message| {
                let call = message.init_root::<Message::Builder>().init_call();
                call.set_question_id(1);
                let promised_answer = call.init_target().init_promised_answer();
                promised_answer.set_question_id(0);
                call.set_interface_id(0x97983392df35cc36);
                call.set_method_id(0);
                let payload = call.init_params();
                let exp = payload.init_content().init_as_struct::<Calculator::Expression::Builder>();
                exp.set_literal(1.23456);

                capnp::serialize::write_message(out_stream, message);

            });

    }
}

pub fn main() {
    use std::io::process;
    use capnp::message::MessageReader;
    use rpc_capnp::{Message, Return, CapDescriptor};

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

    do spawn || {
        let mut r = childStdOut;

        loop {

            let message = capnp::serialize::new_reader(
                &mut r,
                capnp::message::DEFAULT_READER_OPTIONS);
            match message.get_root::<Message::Reader>().which() {
                Some(Message::Return(ret)) => {
                    println!("got a return {}", ret.get_answer_id());
                    match ret.which() {
                        Some(Return::Results(payload)) => {
                            println!("with a payload");
                            let cap_table = payload.get_cap_table();
                            for ii in range(0, cap_table.size()) {
                                match cap_table[ii].which() {
                                    Some(CapDescriptor::None(())) => {}
                                    Some(CapDescriptor::SenderHosted(id)) => {
                                        println!("sender hosted: {}", id);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Some(Return::Exception(_)) => {
                            println!("exception");
                        }
                        _ => {}
                    }
                }
                Some(Message::Unimplemented(_)) => {
                    println!("unimplemented");
                }
                Some(Message::Abort(exc)) => {
                    println!("abort: {}", exc.get_reason());
                }
                None => { println!("Nothing there") }
                _ => {println!("something else") }
            }

        }
    }

    testing::connect(&mut childStdIn);

    p.wait();
}
