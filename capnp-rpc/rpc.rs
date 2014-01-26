/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use capnp::message::{MessageReader};
use capnp::serialize;
use std;
use rpc_capnp::{Message, Return, CapDescriptor};

type QuestionId = u32;
type AnswerId = QuestionId;
type ExportId = u32;
type ImportId = ExportId;

pub struct Question {
    is_awaiting_return : bool
}

pub struct Answer {
    result_exports : ~[ExportId]
}

pub struct Export;

pub struct Import;

pub struct ImportTable<T> {
    slots : ~[T]
}

pub struct ExportTable<T> {
    slots : ~[T]
}

pub struct RpcConnectionState {
    exports : ExportTable<Export>,
    questions : ExportTable<Question>,
    answers : ImportTable<Answer>,
    imports : ImportTable<Import>,
}

impl RpcConnectionState {
    pub fn new() -> RpcConnectionState {
        RpcConnectionState {
            exports : ExportTable { slots : ~[] },
            questions : ExportTable { slots : ~[] },
            answers : ImportTable { slots : ~[] },
            imports : ImportTable { slots : ~[] },
        }
    }
}

pub enum RpcEvent {
    IncomingMessage(~serialize::OwnedSpaceMessageReader),
}

pub fn run_loop (port : std::comm::Port<RpcEvent>) {

    let mut connection_state = RpcConnectionState::new();

    loop {
        match port.recv() {
            IncomingMessage(message) => {
                let root = message.get_root::<Message::Reader>();

                match root.which() {
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
    }
}
