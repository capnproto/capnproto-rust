/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use capnp::any::{AnyPointer};
use capnp::capability;
use capnp::capability::{CallContext, CallContextHook, ClientHook, PipelineHook, PipelineOp, RemotePromise,
                        RequestHook, Request, Server};
use capnp::common;
use capnp::message::{DEFAULT_READER_OPTIONS, MessageReader, MessageBuilder, MallocMessageBuilder};
use capnp::serialize;
use capnp::serialize::{OwnedSpaceMessageReader};

use std;
use std::any::AnyRefExt;
use std::hashmap::HashMap;

use capability::{LocalClient};
use rpc_capnp::{Message, Return, CapDescriptor, MessageTarget};

pub type QuestionId = u32;
pub type AnswerId = QuestionId;
pub type ExportId = u32;
pub type ImportId = ExportId;

pub struct Question {
    chan : std::comm::Chan<~OwnedSpaceMessageReader>,
    is_awaiting_return : bool,
}

pub struct Answer {
    result_exports : ~[ExportId]
}

pub struct Export {
    object : ObjectHandle,
}

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

fn populate_cap_table(message : &mut OwnedSpaceMessageReader,
                      loop_chan : &std::comm::Chan<RpcEvent>) {
    let mut the_cap_table : ~[Option<~ClientHook>] = ~[];
    {
        let root = message.get_root::<Message::Reader>();

        match root.which() {
            Some(Message::Return(ret)) => {
                match ret.which() {
                    Some(Return::Results(payload)) => {
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
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Return::Exception(_e)) => {
                    }
                    _ => {}
                }

            }
            Some(Message::Call(call)) => {
                match call.get_target().which() {
                    Some(MessageTarget::ImportedCap(_import_id)) => {
                    }
                    _ => {
                    }

                }
            }
            Some(Message::Unimplemented(_)) => {
            }
            Some(Message::Abort(_exc)) => {
            }
            None => {
            }
            _ => {
            }
        }
    }
    message.init_cap_table(the_cap_table);

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
        self, inpipe: T, outpipe: U, vat_chan : std::comm::Chan<VatEvent>)
         -> std::comm::Chan<RpcEvent> {

        let (port, chan) = std::comm::Chan::<RpcEvent>::new();

        let listener_chan = chan.clone();

        spawn(proc() {
                let mut r = inpipe;
                loop {
                    let message = box serialize::new_reader(
                        &mut r,
                        DEFAULT_READER_OPTIONS).unwrap();
                    listener_chan.send(IncomingMessage(message));
                }
            });


        let loop_chan = chan.clone();

        let (writer_port, writer_chan) = std::comm::Chan::<~MallocMessageBuilder>::new();

        spawn(proc() {
                let mut w = outpipe;
                loop {
                    let message = match writer_port.recv_opt() {
                        None => break,
                        Some(m) => m,
                    };
                    serialize::write_message(&mut w, message);
                }
            });

        spawn(proc() {
                let RpcConnectionState {mut questions, mut exports, answers : _answers, imports : _imports} = self;
                loop {
                    match port.recv() {
                        IncomingMessage(mut message) => {
                            enum MessageReceiver {
                                Nobody,
                                QuestionReceiver(QuestionId),
                                ExportReceiver(ExportId),
                            }


                            populate_cap_table(message, &loop_chan);
                            let root = message.get_root::<Message::Reader>();
                            let receiver = match root.which() {
                                Some(Message::Unimplemented(_)) => {
                                    println!("unimplemented");
                                    Nobody
                                }
                                Some(Message::Abort(exc)) => {
                                    println!("abort: {}", exc.get_reason());
                                    Nobody
                                }
                                Some(Message::Call(call)) => {
                                    match call.get_target().which() {
                                        Some(MessageTarget::ImportedCap(import_id)) => {
                                            ExportReceiver(import_id)
                                        }
                                        _ => {
                                            fail!("call targets something else");
                                        }
                                    }
                                }

                                Some(Message::Return(ret)) => {
                                    QuestionReceiver(ret.get_answer_id())
                                }
                                Some(Message::Finish(_finish)) => {
                                    println!("finish");
                                    Nobody
                                }
                                Some(Message::Resolve(_resolve)) => {
                                    println!("resolve");
                                    Nobody
                                }
                                Some(Message::Release(_rel)) => {
                                    println!("release");
                                    Nobody
                                }
                                Some(Message::Disembargo(_dis)) => {
                                    println!("disembargo");
                                    Nobody
                                }
                                Some(Message::Save(_save)) => {
                                    Nobody
                                }
                                Some(Message::Restore(restore)) => {
                                    let (port, chan) = std::comm::Chan::new();
                                    vat_chan.send(
                                        VatEventRestore(restore.get_object_id().get_as_text().to_owned(), chan));
                                    let localclient = port.recv().unwrap();
                                    let idx = exports.slots.len();
                                    exports.slots.push(Export { object : localclient.object.clone() });
                                    let mut message = ~MallocMessageBuilder::new_default();
                                    {
                                        let root : Message::Builder = message.init_root();
                                        let ret = root.init_return();
                                        ret.set_answer_id(restore.get_question_id());
                                        let payload = ret.init_results();
                                        payload.init_cap_table(1);
                                        payload.get_cap_table()[0].set_sender_hosted(idx as u32);
                                        payload.get_content().set_as_capability(localclient.copy());
                                    }
                                    writer_chan.send(message);
                                    Nobody
                                }
                                Some(Message::Delete(_delete)) => {
                                    Nobody
                                }
                                Some(Message::Provide(_provide)) => {
                                    Nobody
                                }
                                Some(Message::Accept(_accept)) => {
                                    Nobody
                                }
                                Some(Message::Join(_join)) => {
                                    Nobody
                                }
                                None => {
                                    println!("Nothing there");
                                    Nobody
                                }
                            };
                            match receiver {
                                Nobody => {}
                                QuestionReceiver(id) => {
                                    questions.slots[id].chan.try_send(message);
                                }
                                ExportReceiver(id) => {
                                    exports.slots[id].object.chan.send((message, loop_chan.clone()));
                                }
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
                        NewLocalServer(obj, export_chan) => {
                            let export_id = exports.slots.len() as u32;
                            export_chan.send(export_id);
                            exports.slots.push(Export { object : obj });
                        }
                        ReturnEvent(message) => {
                            writer_chan.send(message);
                        }
                        ShutdownEvent => {
                            break;
                        }
                    }
                }});
        return chan;
    }
}

// HACK
pub enum OwnedCapDescriptor {
    NoDescriptor,
    SenderHosted(ExportId),
    SenderPromise(ExportId),
    ReceiverHosted(ImportId),
    ReceiverAnswer(QuestionId, ~[PipelineOp::Type]),
}

pub struct ImportClient {
    priv channel : std::comm::Chan<RpcEvent>,
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

    fn get_descriptor(&self) -> ~std::any::Any {
        (box ReceiverHosted(self.import_id)) as ~std::any::Any
    }
}

pub struct PipelineClient {
    priv channel : std::comm::Chan<RpcEvent>,
    ops : ~[PipelineOp::Type],
    question_id : QuestionId,
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

    fn get_descriptor(&self) -> ~std::any::Any {
        (box ReceiverAnswer(self.question_id, self.ops.clone())) as ~std::any::Any
    }
}


pub struct RpcRequest {
    priv channel : std::comm::Chan<RpcEvent>,
    priv message : ~MallocMessageBuilder,
}

impl RequestHook for RpcRequest {
    fn message<'a>(&'a mut self) -> &'a mut MallocMessageBuilder {
        &mut *self.message
    }
    fn send(~self) -> RemotePromise<AnyPointer::Reader, AnyPointer::Pipeline> {

        let ~RpcRequest { channel, mut message } = self;

        {
            let cap_table = {
                let mut caps = box [];
                for cap in message.get_cap_table().iter() {
                    match cap {
                        &Some(ref client_hook) => {
                            caps.push(client_hook.get_descriptor())
                        }
                        &None => {}
                    }
                }
                caps
            };
            let root : Message::Builder = message.get_root();
            match root.which() {
                Some(Message::Which::Call(call)) => {
                    let new_cap_table = call.get_params().init_cap_table(cap_table.len());
                    for ii in range(0, cap_table.len()) {
                        match cap_table[ii].as_ref::<OwnedCapDescriptor>() {
                            Some(&NoDescriptor) => {}
                            Some(&ReceiverHosted(import_id)) => {
                                new_cap_table[ii].set_receiver_hosted(import_id);
                            }
                            Some(&ReceiverAnswer(question_id,ref ops)) => {
                                let promised_answer = new_cap_table[ii].init_receiver_answer();
                                promised_answer.set_question_id(question_id);
                                let transform = promised_answer.init_transform(ops.len());
                                for ii in range(0, ops.len()) {
                                    match ops[ii] {
                                        PipelineOp::Noop => transform[ii].set_noop(()),
                                        PipelineOp::GetPointerField(idx) => transform[ii].set_get_pointer_field(idx),
                                    }
                                }
                            }
                            Some(&SenderHosted(export_id)) => {
                                new_cap_table[ii].set_sender_hosted(export_id);
                            }
                            None => {
                                match cap_table[ii].as_ref::<ObjectHandle>() {
                                    Some(obj) => {
                                        let (port, chan) = std::comm::Chan::<ExportId>::new();
                                        channel.send(NewLocalServer(obj.clone(), chan));
                                        let idx = port.recv();
                                        new_cap_table[ii].set_sender_hosted(idx);
                                    }
                                    None => fail!("noncompliant client hook"),
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

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
    channel : std::comm::Chan<RpcEvent>,
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

pub struct RpcCallContext {
    params_message : ~OwnedSpaceMessageReader,
    results_message : ~MallocMessageBuilder,
    rpc_chan : std::comm::Chan<RpcEvent>,
}

impl RpcCallContext {
    pub fn new(params_message : ~OwnedSpaceMessageReader,
               rpc_chan : std::comm::Chan<RpcEvent>) -> RpcCallContext {
        let answer_id = {
            let root : Message::Reader = params_message.get_root();
            match root.which() {
                Some(Message::Call(call)) => {
                    call.get_question_id()
                }
                _ => fail!(),
            }
        };
        let mut results_message = ~MallocMessageBuilder::new_default();
        {
            let root : Message::Builder = results_message.init_root();
            let ret = root.init_return();
            ret.set_answer_id(answer_id);
            ret.init_results();
        }
        RpcCallContext {
            params_message : params_message,
            results_message : results_message,
            rpc_chan : rpc_chan,
        }
    }
}

impl CallContextHook for RpcCallContext {
    fn get<'a>(&'a mut self) -> (AnyPointer::Reader<'a>, AnyPointer::Builder<'a>) {

        let params = {
            let root : Message::Reader = self.params_message.get_root();
            match root.which() {
                Some(Message::Call(call)) => {
                    call.get_params().get_content()
                }
                _ => fail!(),
            }
        };

        let results = {
            let root : Message::Builder = self.results_message.get_root();
            match root.which() {
                Some(Message::Which::Return(ret)) => {
                    match ret.which() {
                        Some(Return::Which::Results(results)) => {
                            results.get_content()
                        }
                        _ => fail!(),
                    }
                }
                _ => fail!(),
            }
        };

        (params, results)
    }
    fn done(~self) {
        let ~RpcCallContext { params_message : _, results_message, rpc_chan} = self;
        rpc_chan.send(ReturnEvent(results_message));
    }
}

pub struct OutgoingMessage {
    message : ~MallocMessageBuilder,
    answer_chan : std::comm::Chan<~OwnedSpaceMessageReader>,
    question_chan : std::comm::Chan<QuestionId>,
}


pub enum RpcEvent {
    IncomingMessage(~serialize::OwnedSpaceMessageReader),
    Outgoing(OutgoingMessage),
    NewLocalServer(ObjectHandle, std::comm::Chan<ExportId>),
    ReturnEvent(~MallocMessageBuilder),
    ShutdownEvent,
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


// ----




#[deriving(Clone)]
pub struct ObjectHandle {
    chan : std::comm::Chan<(~OwnedSpaceMessageReader, std::comm::Chan<RpcEvent>)>,
}

impl ObjectHandle {
    pub fn new(server : ~Server) -> ObjectHandle {
        let (port, chan) =
            std::comm::Chan::<(~OwnedSpaceMessageReader, std::comm::Chan<RpcEvent>)>::new();
        std::task::spawn(proc () {
                let mut server = server;
                loop {
                    let (message, rpc_chan) = match port.recv_opt() {
                        None => break,
                        Some((m,c)) => (m,c),
                    };

                    // XXX
                    let (interface_id, method_id) = {
                        let root : Message::Reader = message.get_root();
                        match root.which() {
                            Some(Message::Call(call)) => (call.get_interface_id(), call.get_method_id()),
                            _ => fail!(),
                        }
                    };
                    let context = CallContext { hook : ~RpcCallContext::new(message, rpc_chan)};
                    server.dispatch_call(interface_id, method_id, context);
                }
            });

        ObjectHandle { chan : chan }
    }
}

pub enum VatEvent {
    VatEventRestore(~str /* XXX */, std::comm::Chan<Option<LocalClient>>),
    VatEventRegister(~str /* XXX */, ~Server),
}

pub struct Vat {
    objects : std::hashmap::HashMap<~str, LocalClient>,
}

impl Vat {
    pub fn new() -> std::comm::Chan<VatEvent> {
        let (port, chan) = std::comm::Chan::<VatEvent>::new();

        std::task::spawn(proc() {
                let mut vat = Vat { objects : std::hashmap::HashMap::new() };

                loop {
                    match port.recv_opt() {
                        Some(VatEventRegister(name, server)) => {
                            vat.objects.insert(name, LocalClient { object : ObjectHandle::new(server)} );
                        }
                        Some(VatEventRestore(name, return_chan)) => {
                            return_chan.send(Some((*vat.objects.get(&name)).clone()));
                        }
                        None => break,
                    }
                }
            });

        chan
    }
}
