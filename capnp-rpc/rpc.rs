/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use capnp::any::{AnyPointer};
use capnp::capability;
use capnp::capability::{RemotePromise, RequestHook, ClientHook};
use capnp::common;
use capnp::message::{DEFAULT_READER_OPTIONS, MessageReader, MessageBuilder, MallocMessageBuilder};
use capnp::serialize;
use capnp::serialize::{OwnedSpaceMessageReader};
use std;
use std::hashmap::HashMap;
use rpc_capnp::{Message, Return, CapDescriptor};

type QuestionId = u32;
type AnswerId = QuestionId;
type ExportId = u32;
type ImportId = ExportId;

pub struct Question {
    chan : std::comm::Chan<~OwnedSpaceMessageReader>,
    is_awaiting_return : bool,
}

pub struct Answer {
    result_exports : ~[ExportId]
}

pub struct Export;

pub struct Import;

pub struct ImportTable<T> {
    slots : HashMap<u32, T>,
}

impl <T> ImportTable<T> {
    pub fn new() -> ImportTable<T> {
        ImportTable { slots : HashMap::new() }
    }
}

pub struct ExportTable<T> {
    slots : ~[T],
}

impl <T> ExportTable<T> {
    pub fn new() -> ExportTable<T> {
        ExportTable { slots : ~[] }
    }

    pub fn next(&mut self) -> u32 {
        fail!()
    }
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
            exports : ExportTable::new(),
            questions : ExportTable::new(),
            answers : ImportTable::new(),
            imports : ImportTable::new(),
        }
    }

    pub fn run<T : std::io::Reader + Send, U : std::io::Writer + Send>(
        self, inpipe: T, outpipe: U)
         -> std::comm::SharedChan<RpcEvent> {

        let (port, chan) = std::comm::SharedChan::<RpcEvent>::new();

        let listener_chan = chan.clone();

        spawn(proc() {
                let mut r = inpipe;
                loop {
                    let message = box serialize::new_reader(
                        &mut r,
                        DEFAULT_READER_OPTIONS);
                    listener_chan.send(IncomingMessage(message));
                }
            });

        let loop_chan = chan.clone();

        spawn(proc() {
                let RpcConnectionState {mut questions, exports, answers, imports} = self;
                let mut outpipe = outpipe;
                loop {
                    match port.recv() {
                        IncomingMessage(mut message) => {
                            let mut the_cap_table : ~[Option<~ClientHook>] = ~[];
                            let mut question = None::<u32>;
                            // populate the cap table
                            {
                                let root = message.get_root::<Message::Reader>();

                                match root.which() {
                                    Some(Message::Return(ret)) => {
                                        println!("got a return with answer id {}", ret.get_answer_id());

                                        match ret.which() {
                                            Some(Return::Results(payload)) => {
                                                println!("with a payload");
                                                let cap_table = payload.get_cap_table();
                                                for ii in range(0, cap_table.size()) {
                                                    match cap_table[ii].which() {
                                                        Some(CapDescriptor::None(())) => {
                                                            the_cap_table.push(None)
                                                        }
                                                        Some(CapDescriptor::SenderHosted(id)) => {
                                                            the_cap_table.push(Some(
                                                                    (box ImportClient {
                                                                            channel : loop_chan.clone(),
                                                                            import_id : id})
                                                                        as ~ClientHook));
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

                                        question = Some(ret.get_answer_id());
                                    }
                                    Some(Message::Unimplemented(_)) => {
                                        println!("unimplemented");
                                    }
                                    Some(Message::Abort(exc)) => {
                                        println!("abort: {}", exc.get_reason());
                                    }
                                    None => {
                                        println!("Nothing there");
                                    }
                                    _ => {
                                        println!("something else");
                                    }
                                }
                            }

                            message.init_cap_table(the_cap_table);

                            match question {
                                Some(id) => {
                                    questions.slots[id].chan.send(message)
                                }
                                None => {}
                            }

                        }
                        OutgoingMessage(mut m, chan) => {
                            let root = m.get_root::<Message::Builder>();
                            // add a question to the question table
                            match root.which() {
                                Some(Message::Which::Return(_)) => {}
                                Some(Message::Which::Call(_)) => {}
                                Some(Message::Which::Restore(res)) => {
                                    res.set_question_id(questions.slots.len() as u32);
                                    questions.slots.push(Question {is_awaiting_return : true,
                                                                   chan : chan} );
                                }
                                _ => {
                                    error!("NONE OF THOSE");
                                }
                            }

                            // send
                            serialize::write_message(&mut outpipe, m);
                        }
                        _ => {
                            println!("got another event");
                        }
                    }
                }});
        return chan;
    }
}

pub struct ImportClient {
    priv channel : std::comm::SharedChan<RpcEvent>,
    import_id : ImportId,
}

impl ClientHook for ImportClient {
    fn copy(&self) -> ~ClientHook {
        (box ImportClient {channel : self.channel.clone(),
                           import_id : self.import_id}) as ~ClientHook
    }

    fn new_call(&self, interface_id : u64, method_id : u16,
                _size_hint : Option<common::MessageSize>)
                -> capability::Request<AnyPointer::Builder, AnyPointer::Reader> {
        let hook = box RpcRequest { channel : self.channel.clone(),
                                    message : box MallocMessageBuilder::new_default() };
        capability::Request::new(hook as ~RequestHook)
    }
}

pub struct RpcRequest {
    priv channel : std::comm::SharedChan<RpcEvent>,
    priv message : ~MallocMessageBuilder
}

impl RequestHook for RpcRequest {
    fn message<'a>(&'a mut self) -> &'a mut MallocMessageBuilder {
        &mut *self.message
    }
    fn send(~self) -> RemotePromise<AnyPointer::Reader> {
        let (port, chan) = std::comm::Chan::<~OwnedSpaceMessageReader>::new();

        let ~RpcRequest { channel, message } = self;
        channel.send(OutgoingMessage(message, chan));

        RemotePromise {port : port}
    }
}

pub enum RpcEvent {
    Nothing,
    IncomingMessage(~serialize::OwnedSpaceMessageReader),
    OutgoingMessage(~MallocMessageBuilder, std::comm::Chan<~OwnedSpaceMessageReader>)
}

