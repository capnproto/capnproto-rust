/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use capnp::any::{AnyPointer};
use capnp::capability;
use capnp::capability::{RemotePromise, RequestHook, ClientHook, PipelineHook, Request, PipelineOp};
use capnp::common;
use capnp::layout::{FromStructReader, FromStructBuilder, HasStructSize};
use capnp::message::{DEFAULT_READER_OPTIONS, MessageReader, MessageBuilder, MallocMessageBuilder};
use capnp::serialize;
use capnp::serialize::{OwnedSpaceMessageReader};
use std;
use std::hashmap::HashMap;
use rpc_capnp::{Message, Return, CapDescriptor, PromisedAnswer};

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

        let (writer_port, writer_chan) = std::comm::Chan::<~MallocMessageBuilder>::new();

        spawn(proc() {
                let mut w = outpipe;
                loop {
                    let message = writer_port.recv();
                    serialize::write_message(&mut w, message);
                }
            });

        spawn(proc() {
                let RpcConnectionState {mut questions, exports, answers, imports} = self;
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
                                            Some(Return::Exception(e)) => {
                                                println!("exception: {}", e.get_reason());
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
                                    questions.slots[id].chan.try_send(message);
                                }
                                None => {}
                            }

                        }
                        Outgoing(OutgoingMessage { message : mut m,
                                                   answer_chan,
                                                   question_chan} ) => {
                            let root = m.get_root::<Message::Builder>();
                            // add a question to the question table
                            match root.which() {
                                Some(Message::Which::Return(_)) => {}
                                Some(Message::Which::Call(call)) => {
                                    call.set_question_id(questions.slots.len() as u32);
                                    questions.slots.push(Question {is_awaiting_return : true,
                                                                   chan : answer_chan} );
                                    question_chan.try_send(call.get_question_id());
                                }
                                Some(Message::Which::Restore(res)) => {
                                    res.set_question_id(questions.slots.len() as u32);
                                    questions.slots.push(Question {is_awaiting_return : true,
                                                                   chan : answer_chan} );
                                    question_chan.try_send(res.get_question_id());
                                }
                                _ => {
                                    error!("NONE OF THOSE");
                                }
                            }

                            // send
                            writer_chan.send(m);
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
                -> capability::Request<AnyPointer::Builder, AnyPointer::Reader, AnyPointer::Pipeline> {
        let mut message = box MallocMessageBuilder::new_default();
        {
            let root : Message::Builder = message.get_root();
            let call = root.init_call();
            call.set_interface_id(interface_id);
            call.set_method_id(method_id);
            let target = call.init_target();
            target.set_imported_cap(self.import_id);
        }
        let hook = box RpcRequest { channel : self.channel.clone(),
                                    message : message };
        Request::new(hook as ~RequestHook)
    }
}

pub struct PipelineClient {
    priv channel : std::comm::SharedChan<RpcEvent>,
    ops : ~[PipelineOp::Type],
    question_id : ExportId,
}

impl ClientHook for PipelineClient {
    fn copy(&self) -> ~ClientHook {
        (~PipelineClient { channel : self.channel.clone(),
                           ops : self.ops.clone(),
                           question_id : self.question_id,
            }) as ~ClientHook
    }

    fn new_call(&self, interface_id : u64, method_id : u16,
                _size_hint : Option<common::MessageSize>)
                -> capability::Request<AnyPointer::Builder, AnyPointer::Reader, AnyPointer::Pipeline> {
        let mut message = box MallocMessageBuilder::new_default();
        {
            let root : Message::Builder = message.get_root();
            let call = root.init_call();
            call.set_interface_id(interface_id);
            call.set_method_id(method_id);
            let target = call.init_target();
            let promised_answer = target.init_promised_answer();
            promised_answer.set_question_id(self.question_id);
            let transform = promised_answer.init_transform(self.ops.len());
            for ii in range(0, self.ops.len()) {
                match self.ops[ii] {
                    PipelineOp::Noop => transform[ii].set_noop(()),
                    PipelineOp::GetPointerField(idx) => transform[ii].set_get_pointer_field(idx),
                }
            }
        }
        let hook = box RpcRequest { channel : self.channel.clone(),
                                    message : message };
        Request::new(hook as ~RequestHook)
    }
}

pub struct RpcRequest {
    priv channel : std::comm::SharedChan<RpcEvent>,
    priv message : ~MallocMessageBuilder,
}

impl RequestHook for RpcRequest {
    fn message<'a>(&'a mut self) -> &'a mut MallocMessageBuilder {
        &mut *self.message
    }
    fn send(~self) -> RemotePromise<AnyPointer::Reader, AnyPointer::Pipeline> {

        let ~RpcRequest { channel, message } = self;
        let (event, answer_port, question_port) = RpcEvent::new_outgoing(message);
        channel.send(event);

        let question_id = question_port.recv();

        let pipeline = ~RpcPipeline {channel : channel, question_id : question_id};
        let typeless = AnyPointer::Pipeline::new(pipeline as ~PipelineHook);

        RemotePromise {answer_port : answer_port, answer_result : None,
                       pipeline : typeless  }
    }
}

pub struct RpcPipeline {
    channel : std::comm::SharedChan<RpcEvent>,
    question_id : ExportId,
}

impl PipelineHook for RpcPipeline {
    fn copy(&self) -> ~PipelineHook {
        (~RpcPipeline { channel : self.channel.clone(),
                        question_id : self.question_id }) as ~PipelineHook
    }
    fn get_pipelined_cap(&self, ops : ~[PipelineOp::Type]) -> ~ClientHook {
        (~PipelineClient { channel : self.channel.clone(),
                           ops : ops,
                           question_id : self.question_id,
        }) as ~ClientHook
    }
}

pub trait InitParams<'a, T> {
    fn init_params(&'a mut self) -> T;
}

impl <'a, Params : FromStructBuilder<'a> + HasStructSize, Results, Pipeline> InitParams<'a, Params>
for Request<Params, Results, Pipeline> {
    fn init_params(&'a mut self) -> Params {
        let message : Message::Builder = self.hook.message().get_root();
        match message.which() {
            Some(Message::Which::Call(call)) => {
                let params = call.init_params();
                params.get_content().init_as_struct()
            }
            _ => fail!(),
        }
    }
}

pub trait WaitForContent<'a, T> {
    fn wait(&'a mut self) -> T;
}

impl <'a, Results : FromStructReader<'a>, Pipeline> WaitForContent<'a, Results>
for RemotePromise<Results, Pipeline> {
    fn wait(&'a mut self) -> Results {
        // XXX should check that it's not already been received.
        let message = self.answer_port.recv();
        self.answer_result = Some(message);
        match self.answer_result {
            None => unreachable!(),
            Some(ref message) => {
                let root : Message::Reader = message.get_root();
                match root.which() {
                    Some(Message::Return(ret)) => {
                        match ret.which() {
                            Some(Return::Results(res)) => {
                                res.get_content().get_as_struct()
                            }
                            _ => fail!(),
                        }
                    }
                    _ => {fail!()}
                }
            }
        }
    }
}


pub struct OutgoingMessage {
    message : ~MallocMessageBuilder,
    answer_chan : std::comm::Chan<~OwnedSpaceMessageReader>,
    question_chan : std::comm::Chan<ExportId>,
}


pub enum RpcEvent {
    Nothing,
    IncomingMessage(~serialize::OwnedSpaceMessageReader),
    Outgoing(OutgoingMessage),
}


impl RpcEvent {
    pub fn new_outgoing(message : ~MallocMessageBuilder)
                        -> (RpcEvent, std::comm::Port<~OwnedSpaceMessageReader>,
                            std::comm::Port<ExportId>) {
        let (answer_port, answer_chan) = std::comm::Chan::<~OwnedSpaceMessageReader>::new();

        let (question_port, question_chan) = std::comm::Chan::<ExportId>::new();

        (Outgoing(OutgoingMessage{ message : message,
                                   answer_chan : answer_chan,
                                   question_chan : question_chan }),
         answer_port,
         question_port)
    }
}
