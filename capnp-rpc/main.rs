
#[crate_id="capnp-rpc"];
#[crate_type="bin"];

extern mod capnp;

//pub mod calculator_capnp;
pub mod rpc_capnp;
//pub mod rpc-twoparty_capnp;

pub mod testing {
    use capnp;
    use rpc_capnp::{Message, Return, CapDescriptor};
    use std::io::net;
    use std;

    pub fn connect(addr_str : &str) {
        let sockaddr = match std::from_str::FromStr::from_str(addr_str) {
            None => fail!("could not parse socket address: {}", addr_str),
            Some(a) => a
        };

        let mut stream = net::tcp::TcpStream::connect(sockaddr);

        capnp::message::MessageBuilder::new_default(|message| {
                let restore = message.init_root::<Message::Builder>().init_restore();
                restore.set_question_id(0);
                restore.init_object_id().set_as_text("calculator");

                capnp::serialize::write_message(&mut stream, message);
            });

        capnp::serialize::InputStreamMessageReader::new(
            &mut stream,
            capnp::message::DEFAULT_READER_OPTIONS,
            |message| {
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
                            _ => {}
                        }
                    }
                    _ => {println!("something else") }
                }
            });

        capnp::message::MessageBuilder::new_default(|message| {
                let call = message.init_root::<Message::Builder>().init_call();
                call.set_question_id(1);
                let promised_answer = call.init_target().init_promised_answer();
                promised_answer.set_question_id(0);
                call.set_interface_id(0x97983392df35cc36);
                call.set_method_id(0);
                let payload = call.init_params();
                // Hm... I need EvaluateParams.

            });

    }
}

pub fn main() {
    testing::connect(std::os::args()[1]);
}
