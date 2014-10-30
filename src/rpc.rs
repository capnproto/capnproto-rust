/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use capnp::{any_pointer};
use capnp::capability;
use capnp::capability::{CallContextHook, ClientHook, PipelineHook, pipeline_op, ResultFuture,
                        RequestHook, Request, ResponseHook};
use capnp::common;
use capnp::{ReaderOptions, MessageReader, BuilderOptions, MessageBuilder, MallocMessageBuilder};
use capnp::serialize;
use capnp::OwnedSpaceMessageReader;

use std::any::AnyRefExt;
use std::vec::Vec;
use std::collections::hashmap::HashMap;
use std::collections::priority_queue::PriorityQueue;
use sync::{Arc, Mutex};

use rpc_capnp::{message, return_, cap_descriptor, message_target, payload, promised_answer};

pub type QuestionId = u32;
pub type AnswerId = QuestionId;
pub type ExportId = u32;
pub type ImportId = ExportId;

pub struct Question {
    chan : ::std::comm::Sender<Box<ResponseHook+Send>>,
    is_awaiting_return : bool,
    ref_counter : ::std::comm::Receiver<()>,
}

impl Question {
    pub fn new(sender : ::std::comm::Sender<Box<ResponseHook+Send>>) -> (Question, ::std::comm::Sender<()>) {
        let (tx, rx) = ::std::comm::channel::<()>();
        (Question {
            chan : sender,
            is_awaiting_return : true,
            ref_counter : rx,
        },
         tx)
    }
}

pub struct QuestionRef {
    pub id : u32,

    // piggy back to get ref counting. we never actually send on this channel.
    ref_count : ::std::comm::Sender<()>,

    rpc_chan : ::std::comm::Sender<RpcEvent>,
}

impl QuestionRef {
    pub fn new(id : u32, ref_count : ::std::comm::Sender<()>,
               rpc_chan : ::std::comm::Sender<RpcEvent>) -> QuestionRef {
        QuestionRef { id : id,
                      ref_count : ref_count,
                      rpc_chan : rpc_chan }
    }
}

impl Clone for QuestionRef {
    fn clone(&self) -> QuestionRef {
        QuestionRef { id : self.id,
                      ref_count : self.ref_count.clone(),
                      rpc_chan : self.rpc_chan.clone()}
    }
}

pub enum AnswerStatus {
    AnswerStatusSent(Box<MallocMessageBuilder>),
    AnswerStatusPending(Vec<(u64, u16, Vec<pipeline_op::Type>, Box<CallContextHook+Send>)>),
}

pub struct AnswerRef {
    status : Arc<Mutex<AnswerStatus>>,
}

impl Clone for AnswerRef {
    fn clone(&self) -> AnswerRef {
        AnswerRef {
            status : self.status.clone(),
        }
    }
}

impl AnswerRef {
    pub fn new() -> AnswerRef {
        AnswerRef {
            status : Arc::new(Mutex::new(AnswerStatusPending(Vec::new()))),
        }
    }

    fn do_call(answer_message : &mut Box<MallocMessageBuilder>, interface_id : u64, method_id : u16,
               ops : Vec<pipeline_op::Type>, context : Box<CallContextHook+Send>) {
        let root : message::Builder = answer_message.get_root();
        match root.which() {
            Some(message::Return(ret)) => {
                match ret.which() {
                    Some(return_::Results(payload)) => {
                        let hook = payload.get_content().as_reader().
                            get_pipelined_cap(ops.as_slice());
                        hook.call(interface_id, method_id, context);
                    }
                    Some(return_::Exception(_exc)) => {
                        // TODO
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    pub fn receive(&mut self, interface_id : u64, method_id : u16,
                   ops : Vec<pipeline_op::Type>, context : Box<CallContextHook+Send>) {
        match self.status.lock().deref_mut() {
            &AnswerStatusSent(ref mut answer_message) => {
                AnswerRef::do_call(answer_message, interface_id, method_id, ops, context);
            }
            &AnswerStatusPending(ref mut waiters) => {
                waiters.push((interface_id, method_id, ops, context));
            }
        }
    }

    pub fn sent(&mut self, mut message : Box<MallocMessageBuilder>) {
        match self.status.lock().deref_mut() {
            &AnswerStatusSent(_) => {panic!()}
            &AnswerStatusPending(ref mut waiters) => {
                waiters.reverse();
                while waiters.len() > 0 {
                    let (interface_id, method_id, ops, context) = match waiters.pop() {
                        Some(r) => r,
                        None => panic!(),
                    };
                    AnswerRef::do_call(&mut message, interface_id, method_id, ops, context);
                }
            }
        }
        *self.status.lock() = AnswerStatusSent(message);
    }


}

pub struct Answer {
    answer_ref : AnswerRef,
    result_exports : Vec<ExportId>,
}

impl Answer {
    pub fn new() -> Answer {
        Answer {
            answer_ref : AnswerRef::new(),
            result_exports : Vec::new(),
        }
    }
}

pub struct Export {
    hook : Box<ClientHook+Send>,
    reference_count : i32,
}

impl Export {
    pub fn new(hook : Box<ClientHook+Send>) -> Export {
        Export { hook : hook, reference_count : 0 }
    }
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

#[deriving(PartialEq, Eq)]
struct ReverseU32 { val : u32 }

impl ::core::cmp::Ord for ReverseU32 {
    fn cmp(&self, other : &ReverseU32) -> Ordering {
        if self.val > other.val { Less }
        else if self.val < other.val { Greater }
        else { Equal }
    }
}

impl ::core::cmp::PartialOrd for ReverseU32 {
    fn partial_cmp(&self, other : &ReverseU32) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}


pub struct ExportTable<T> {
    slots : Vec<Option<T>>,

    // prioritize lower values
    free_ids : PriorityQueue<ReverseU32>,
}

impl <T> ExportTable<T> {
    pub fn new() -> ExportTable<T> {
        ExportTable { slots : Vec::new(),
                      free_ids : PriorityQueue::new() }
    }

    pub fn erase(&mut self, id : u32) {
        *self.slots.get_mut(id as uint) = None;
        self.free_ids.push(ReverseU32 { val : id } );
    }

    pub fn push(&mut self, val : T) -> u32 {
        match self.free_ids.pop() {
            Some(ReverseU32 { val : id }) => {
                *self.slots.get_mut(id as uint) = Some(val);
                id
            }
            None => {
                self.slots.push(Some(val));
                self.slots.len() as u32 - 1
            }
        }
    }
}

pub trait SturdyRefRestorer {
    fn restore(&self, _obj_id : any_pointer::Reader) -> Option<Box<ClientHook+Send>> { None }
}

impl SturdyRefRestorer for () { }


pub struct RpcConnectionState {
    exports : ExportTable<Export>,
    questions : ExportTable<Question>,
    answers : ImportTable<Answer>,
    imports : ImportTable<Import>,
}

fn client_hooks_of_payload(payload : payload::Reader,
                           rpc_chan : &::std::comm::Sender<RpcEvent>,
                           answers : &ImportTable<Answer>) -> Vec<Option<Box<ClientHook+Send>>> {
    let mut result = Vec::new();
    for cap in payload.get_cap_table().iter() {
        match cap.which() {
            Some(cap_descriptor::None(())) => {
                result.push(None)
            }
            Some(cap_descriptor::SenderHosted(id)) => {
                result.push(Some(
                        (box ImportClient {
                                channel : rpc_chan.clone(),
                                import_id : id})
                            as Box<ClientHook+Send>));
            }
            Some(cap_descriptor::SenderPromise(_id)) => {
                println!("warning: SenderPromise is unimplemented");
                result.push(None);
            }
            Some(cap_descriptor::ReceiverHosted(_id)) => {
                panic!()
            }
            Some(cap_descriptor::ReceiverAnswer(promised_answer)) => {
                result.push(Some(
                        (box PromisedAnswerClient {
                                rpc_chan : rpc_chan.clone(),
                                ops : get_pipeline_ops(promised_answer),
                                answer_ref : answers.slots[promised_answer.get_question_id()]
                                .answer_ref.clone(),
                                } as Box<ClientHook+Send>)));
            }
            Some(cap_descriptor::ThirdPartyHosted(_)) => {
                panic!()
            }
            None => { panic!("unknown cap descriptor")}
        }
    }
    result
}

fn populate_cap_table(message : &mut OwnedSpaceMessageReader,
                      rpc_chan : &::std::comm::Sender<RpcEvent>,
                      answers : &ImportTable<Answer>) {
    let mut the_cap_table : Vec<Option<Box<ClientHook+Send>>> = Vec::new();
    {
        let root = message.get_root::<message::Reader>();

        match root.which() {
            Some(message::Return(ret)) => {
                match ret.which() {
                    Some(return_::Results(payload)) => {
                        the_cap_table = client_hooks_of_payload(payload, rpc_chan, answers);
                    }
                    Some(return_::Exception(_e)) => {
                    }
                    _ => {}
                }

            }
            Some(message::Call(call)) => {
               the_cap_table = client_hooks_of_payload(call.get_params(), rpc_chan, answers);
            }
            Some(message::Unimplemented(_)) => {
            }
            Some(message::Abort(_exc)) => {
            }
            None => {
            }
            _ => {
            }
        }
    }
    message.init_cap_table(the_cap_table);
}

fn get_pipeline_ops(promised_answer : promised_answer::Reader) -> Vec<pipeline_op::Type> {
    let mut result = Vec::new();
    for op in promised_answer.get_transform().iter() {
        match op.which() {
            Some(promised_answer::op::Noop(())) => result.push(pipeline_op::Noop),
            Some(promised_answer::op::GetPointerField(idx)) => result.push(pipeline_op::GetPointerField(idx)),
            None => {}
        }
    }
    return result;
}

fn finish_question<W : ::std::io::Writer>(questions : &mut ExportTable<Question>,
                                        outpipe : &mut W,
                                        id : u32) {
    questions.erase(id);

    let mut finish_message = box MallocMessageBuilder::new_default();
    {
        let root : message::Builder = finish_message.init_root();
        let finish = root.init_finish();
        finish.set_question_id(id);
        finish.set_release_result_caps(false);
    }

    serialize::write_message(outpipe, &*finish_message).is_ok();
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

    pub fn run<T : ::std::io::Reader + Send, U : ::std::io::Writer + Send, V : SturdyRefRestorer + Send>(
        self, inpipe: T, outpipe: U, restorer : V)
         -> ::std::comm::Sender<RpcEvent> {

        let (result_rpc_chan, port) = ::std::comm::channel::<RpcEvent>();

        let listener_chan = result_rpc_chan.clone();

        spawn(proc() {
                let mut r = inpipe;
                loop {
                    match serialize::new_reader(
                        &mut r,
                        *ReaderOptions::new().fail_fast(false)) {
                        Err(_e) => { listener_chan.send_opt(ShutdownEvent).is_ok(); break; }
                        Ok(message) => {
                            listener_chan.send_opt(IncomingMessage(box message)).is_ok();
                        }
                    }
                }
            });

        let rpc_chan = result_rpc_chan.clone();

        spawn(proc() {
            let RpcConnectionState {mut questions, mut exports, mut answers, imports : _imports} = self;
            let mut outpipe = outpipe;
            loop {
                match port.recv() {
                    IncomingMessage(mut message) => {
                        enum MessageReceiver {
                            Nobody,
                            QuestionReceiver(QuestionId),
                            ExportReceiver(ExportId),
                            PromisedAnswerReceiver(AnswerId, Vec<pipeline_op::Type>),
                        }

                        populate_cap_table(&mut *message, &rpc_chan, &answers);
                        let receiver = match message.get_root::<message::Reader>().which() {
                            Some(message::Unimplemented(_)) => {
                                println!("unimplemented");
                                Nobody
                            }
                            Some(message::Abort(exc)) => {
                                println!("abort: {}", exc.get_reason());
                                Nobody
                            }
                            Some(message::Call(call)) => {
                                match call.get_target().which() {
                                    Some(message_target::ImportedCap(import_id)) => {
                                        ExportReceiver(import_id)
                                    }
                                    Some(message_target::PromisedAnswer(promised_answer)) => {
                                        PromisedAnswerReceiver(
                                            promised_answer.get_question_id(),
                                            get_pipeline_ops(promised_answer))
                                    }
                                    None => {
                                        panic!("call targets something else");
                                    }
                                }
                            }

                            Some(message::Return(ret)) => {
                                QuestionReceiver(ret.get_answer_id())
                            }
                            Some(message::Finish(finish)) => {
                                println!("finish");
                                answers.slots.remove(&finish.get_question_id());
                                finish.get_release_result_caps();

                                Nobody
                            }
                            Some(message::Resolve(_resolve)) => {
                                println!("resolve");
                                Nobody
                            }
                            Some(message::Release(rel)) => {
                                if rel.get_reference_count() == 1 {
                                    exports.erase(rel.get_id());
                                } else {
                                    println!("warning: release count = {}", rel.get_reference_count());
                                }
                                Nobody
                            }
                            Some(message::Disembargo(_dis)) => {
                                println!("disembargo");
                                Nobody
                            }
                            Some(message::Save(_save)) => {
                                Nobody
                            }
                            Some(message::Restore(restore)) => {
                                let clienthook = restorer.restore(restore.get_object_id()).unwrap();
                                let idx = exports.push(Export::new(clienthook.copy()));

                                let answer_id = restore.get_question_id();
                                let mut message = box MallocMessageBuilder::new_default();
                                {
                                    let root : message::Builder = message.init_root();
                                    let ret = root.init_return();
                                    ret.set_answer_id(answer_id);
                                    let payload = ret.init_results();
                                    payload.init_cap_table(1);
                                    payload.get_cap_table().get(0).set_sender_hosted(idx as u32);
                                    payload.get_content().set_as_capability(clienthook);

                                }
                                answers.slots.insert(answer_id, Answer::new());

                                serialize::write_message(&mut outpipe, &*message).is_ok();
                                answers.slots.get_mut(&answer_id).answer_ref.sent(message);

                                Nobody
                            }
                            Some(message::Delete(_delete)) => {
                                Nobody
                            }
                            Some(message::Provide(_provide)) => {
                                Nobody
                            }
                            Some(message::Accept(_accept)) => {
                                Nobody
                            }
                            Some(message::Join(_join)) => {
                                Nobody
                            }
                            None => {
                                println!("unknown message");
                                Nobody
                            }
                        };

                        fn get_call_ids(message : &OwnedSpaceMessageReader) -> (QuestionId, u64, u16) {
                            let root : message::Reader = message.get_root();
                            match root.which() {
                                Some(message::Call(call)) =>
                                    (call.get_question_id(), call.get_interface_id(), call.get_method_id()),
                                _ => panic!(),
                            }
                        }

                        match receiver {
                            Nobody => {}
                            QuestionReceiver(id) => {
                                let erase_it = match questions.slots.get_mut(id as uint) {
                                    &Some(ref mut q) => {
                                        q.chan.send_opt(
                                            box RpcResponse::new(message) as Box<ResponseHook+Send>).is_ok();
                                        q.is_awaiting_return = false;
                                        match q.ref_counter.try_recv() {
                                            Err(::std::comm::Disconnected) => {
                                                true
                                            }
                                            _ => {false}
                                        }
                                    }
                                    &None => {
                                        // XXX Todo
                                        panic!()
                                    }
                                };
                                if erase_it {
                                    finish_question(&mut questions, &mut outpipe, id);
                                }
                            }
                            ExportReceiver(id) => {
                                let (answer_id, interface_id, method_id) = get_call_ids(&*message);
                                let context =
                                    box RpcCallContext::new(message, rpc_chan.clone()) as Box<CallContextHook+Send>;

                                answers.slots.insert(answer_id, Answer::new());
                                match exports.slots[id as uint] {
                                    Some(ref ex) => {
                                        ex.hook.call(interface_id, method_id, context);
                                    }
                                    None => {
                                        // XXX todo
                                        panic!()
                                    }
                                }
                            }
                            PromisedAnswerReceiver(id, ops) => {
                                let (answer_id, interface_id, method_id) = get_call_ids(&*message);
                                let context =
                                    box RpcCallContext::new(message, rpc_chan.clone()) as Box<CallContextHook+Send>;

                                answers.slots.insert(answer_id, Answer::new());
                                answers.slots.get_mut(&id).answer_ref
                                    .receive(interface_id, method_id, ops, context);
                            }
                        }

                    }
                    Outgoing(OutgoingMessage { message : mut m,
                                               answer_chan,
                                               question_chan} ) => {
                        {
                            let root = m.get_root::<message::Builder>();
                            // add a question to the question table
                            match root.which() {
                                Some(message::Return(_)) => {}
                                Some(message::Call(call)) => {
                                    let (question, ref_count) = Question::new(answer_chan);
                                    let id = questions.push(question);
                                    call.set_question_id(id);
                                    let qref = QuestionRef::new(id, ref_count, rpc_chan.clone());
                                    if !question_chan.send_opt(qref).is_ok() { panic!() }
                                }
                                Some(message::Restore(res)) => {
                                    let (question, ref_count) = Question::new(answer_chan);
                                    let id = questions.push(question);
                                    res.set_question_id(id);
                                    let qref = QuestionRef::new(id, ref_count, rpc_chan.clone());
                                    if !question_chan.send_opt(qref).is_ok() { panic!() }
                                }
                                _ => {
                                    panic!("NONE OF THOSE");
                                }
                            }
                        }

                        serialize::write_message(&mut outpipe, &*m).is_ok();
                    }
                    NewLocalServer(clienthook, export_chan) => {
                        let export_id = exports.push(Export::new(clienthook));
                        export_chan.send(export_id);
                    }
                    DoneWithQuestion(id) => {

                        // This isn't used anywhere yet.
                        // The idea is that when the last reference to a question
                        // is erased, this event will be triggered.

                        let erase_it = match questions.slots[id as uint] {
                            Some(ref q) if q.is_awaiting_return => {
                                true
                            }
                            _ => {false}
                        };
                        if erase_it {
                            finish_question(&mut questions, &mut outpipe, id);
                        }
                    }
                    ReturnEvent(mut message) => {
                        serialize::write_message(&mut outpipe, &*message).is_ok();

                        let answer_id_opt =
                            match message.get_root::<message::Builder>().which() {
                                Some(message::Return(ret)) => {
                                    Some(ret.get_answer_id())
                                }
                                _ => {None}
                            };

                        match answer_id_opt {
                            Some(answer_id) => {
                                answers.slots.get_mut(&answer_id).answer_ref.sent(message)
                            }
                            _ => {}
                        }
                    }
                    ShutdownEvent => {
                        break;
                    }
                }}});
             return result_rpc_chan;
         }
}

// HACK
pub enum OwnedCapDescriptor {
    NoDescriptor,
    SenderHosted(ExportId),
    SenderPromise(ExportId),
    ReceiverHosted(ImportId),
    ReceiverAnswer(QuestionId, Vec<pipeline_op::Type>),
}

pub struct ImportClient {
    channel : ::std::comm::Sender<RpcEvent>,
    pub import_id : ImportId,
}

impl ClientHook for ImportClient {
    fn copy(&self) -> Box<ClientHook+Send> {
        (box ImportClient {channel : self.channel.clone(),
                           import_id : self.import_id}) as Box<ClientHook+Send>
    }

    fn new_call(&self, interface_id : u64, method_id : u16,
                _size_hint : Option<common::MessageSize>)
                -> capability::Request<any_pointer::Builder, any_pointer::Reader, any_pointer::Pipeline> {
        let mut message = box MallocMessageBuilder::new(*BuilderOptions::new().fail_fast(false));
        {
            let root : message::Builder = message.get_root();
            let call = root.init_call();
            call.set_interface_id(interface_id);
            call.set_method_id(method_id);
            let target = call.init_target();
            target.set_imported_cap(self.import_id);
        }
        let hook = box RpcRequest { channel : self.channel.clone(),
                                    message : message,
                                    question_ref : None};
        Request::new(hook as Box<RequestHook+Send>)
    }

    fn call(&self, _interface_id : u64, _method_id : u16, _context : Box<CallContextHook>) {
        panic!()
    }

    fn get_descriptor(&self) -> Box<::std::any::Any+'static> {
        (box ReceiverHosted(self.import_id)) as Box<::std::any::Any+'static>
    }
}

pub struct PipelineClient {
    channel : ::std::comm::Sender<RpcEvent>,
    pub ops : Vec<pipeline_op::Type>,
    pub question_ref : QuestionRef,
}

impl ClientHook for PipelineClient {
    fn copy(&self) -> Box<ClientHook+Send> {
        (box PipelineClient { channel : self.channel.clone(),
                              ops : self.ops.clone(),
                              question_ref : self.question_ref.clone(),
            }) as Box<ClientHook+Send>
    }

    fn new_call(&self, interface_id : u64, method_id : u16,
                _size_hint : Option<common::MessageSize>)
                -> capability::Request<any_pointer::Builder, any_pointer::Reader, any_pointer::Pipeline> {
        let mut message = box MallocMessageBuilder::new(*BuilderOptions::new().fail_fast(false));
        {
            let root : message::Builder = message.get_root();
            let call = root.init_call();
            call.set_interface_id(interface_id);
            call.set_method_id(method_id);
            let target = call.init_target();
            let promised_answer = target.init_promised_answer();
            promised_answer.set_question_id(self.question_ref.id);
            let transform = promised_answer.init_transform(self.ops.len() as u32);
            for ii in range(0, self.ops.len()) {
                match self.ops.as_slice()[ii] {
                    pipeline_op::Noop => transform.get(ii as u32).set_noop(()),
                    pipeline_op::GetPointerField(idx) => transform.get(ii as u32).set_get_pointer_field(idx),
                }
            }
        }
        let hook = box RpcRequest { channel : self.channel.clone(),
                                    message : message,
                                    question_ref : Some(self.question_ref.clone())};
        Request::new(hook as Box<RequestHook+Send>)
    }

    fn call(&self, _interface_id : u64, _method_id : u16, _context : Box<CallContextHook>) {
        panic!()
    }

    fn get_descriptor(&self) -> Box<::std::any::Any+'static> {
        (box ReceiverAnswer(self.question_ref.id, self.ops.clone())) as Box<::std::any::Any+'static>
    }
}

pub struct PromisedAnswerClient {
    rpc_chan : ::std::comm::Sender<RpcEvent>,
    ops : Vec<pipeline_op::Type>,
    answer_ref : AnswerRef,
}

impl ClientHook for PromisedAnswerClient {
    fn copy(&self) -> Box<ClientHook+Send> {
        (box PromisedAnswerClient { rpc_chan : self.rpc_chan.clone(),
                                 ops : self.ops.clone(),
                                 answer_ref : self.answer_ref.clone(),
            }) as Box<ClientHook+Send>
    }

    fn new_call(&self, interface_id : u64, method_id : u16,
                _size_hint : Option<common::MessageSize>)
                -> capability::Request<any_pointer::Builder, any_pointer::Reader, any_pointer::Pipeline> {
        let mut message = box MallocMessageBuilder::new(*BuilderOptions::new().fail_fast(false));
        {
            let root : message::Builder = message.get_root();
            let call = root.init_call();
            call.set_interface_id(interface_id);
            call.set_method_id(method_id);
        }

        let hook = box PromisedAnswerRpcRequest { rpc_chan : self.rpc_chan.clone(),
                                                  message : message,
                                                  answer_ref : self.answer_ref.clone(),
                                                  ops : self.ops.clone() };
        Request::new(hook as Box<RequestHook+Send>)
    }

    fn call(&self, _interface_id : u64, _method_id : u16, _context : Box<CallContextHook>) {
        panic!()
    }

    fn get_descriptor(&self) -> Box<::std::any::Any+'static> {
        panic!()
    }
}


fn write_outgoing_cap_table(rpc_chan : &::std::comm::Sender<RpcEvent>, message : &mut MallocMessageBuilder) {
    fn write_payload(rpc_chan : &::std::comm::Sender<RpcEvent>, cap_table : & [Box<::std::any::Any>],
                     payload : payload::Builder) {
        let new_cap_table = payload.init_cap_table(cap_table.len() as u32);
        for ii in range::<u32>(0, cap_table.len() as u32) {
            match cap_table[ii as uint].downcast_ref::<OwnedCapDescriptor>() {
                Some(&NoDescriptor) => {}
                Some(&ReceiverHosted(import_id)) => {
                    new_cap_table.get(ii).set_receiver_hosted(import_id);
                }
                Some(&ReceiverAnswer(question_id,ref ops)) => {
                    let promised_answer = new_cap_table.get(ii).init_receiver_answer();
                    promised_answer.set_question_id(question_id);
                    let transform = promised_answer.init_transform(ops.len() as u32);
                    for jj in range(0, ops.len()) {
                        match ops.as_slice()[jj] {
                            pipeline_op::Noop => transform.get(jj as u32).set_noop(()),
                            pipeline_op::GetPointerField(idx) => transform.get(jj as u32).set_get_pointer_field(idx),
                        }
                    }
                }
                Some(&SenderHosted(export_id)) => {
                    new_cap_table.get(ii).set_sender_hosted(export_id);
                }
                None => {
                    match cap_table[ii as uint].downcast_ref::<Box<ClientHook+Send>>() {
                        Some(clienthook) => {
                            let (chan, port) = ::std::comm::channel::<ExportId>();
                            rpc_chan.send(NewLocalServer(clienthook.copy(), chan));
                            let idx = port.recv();
                            new_cap_table.get(ii).set_sender_hosted(idx);
                        }
                        None => panic!("noncompliant client hook"),
                    }
                }
                _ => {}
            }
        }
    }
    let cap_table = {
        let mut caps = Vec::new();
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
    let root : message::Builder = message.get_root();
    match root.which() {
        Some(message::Call(call)) => {
            write_payload(rpc_chan, cap_table.as_slice(), call.get_params())
        }
        Some(message::Return(ret)) => {
            match ret.which() {
                Some(return_::Results(payload)) => {
                    write_payload(rpc_chan, cap_table.as_slice(), payload);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

pub struct RpcResponse {
    message : Box<OwnedSpaceMessageReader>,
}

impl RpcResponse {
    pub fn new(message : Box<OwnedSpaceMessageReader>) -> RpcResponse {
        RpcResponse { message : message }
    }
}

impl ResponseHook for RpcResponse {
    fn get<'a>(&'a mut self) -> any_pointer::Reader<'a> {
        self.message.get_root_internal()
    }
}

pub struct RpcRequest {
    channel : ::std::comm::Sender<RpcEvent>,
    message : Box<MallocMessageBuilder>,
    question_ref : Option<QuestionRef>,
}

impl RequestHook for RpcRequest {
    fn message<'a>(&'a mut self) -> &'a mut MallocMessageBuilder {
        &mut *self.message
    }
    fn send<'a>(self : Box<RpcRequest>) -> ResultFuture<any_pointer::Reader<'a>, any_pointer::Pipeline> {
        let tmp = *self;
        let RpcRequest { channel, mut message, question_ref : _ } = tmp;
        write_outgoing_cap_table(&channel, &mut *message);

        let (outgoing, answer_port, question_port) = RpcEvent::new_outgoing(message);
        channel.send(Outgoing(outgoing));

        let question_ref = question_port.recv();

        let pipeline = box RpcPipeline {channel : channel, question_ref : question_ref};
        let typeless = any_pointer::Pipeline::new(pipeline as Box<PipelineHook+Send>);

        ResultFuture {answer_port : answer_port, answer_result : Err(()) /* XXX */,
                       pipeline : typeless  }
    }
}

pub struct PromisedAnswerRpcRequest {
    rpc_chan : ::std::comm::Sender<RpcEvent>,
    message : Box<MallocMessageBuilder>,
    answer_ref : AnswerRef,
    ops : Vec<pipeline_op::Type>,
}

impl RequestHook for PromisedAnswerRpcRequest {
    fn message<'a>(&'a mut self) -> &'a mut MallocMessageBuilder {
        &mut *self.message
    }
    fn send<'a>(self : Box<PromisedAnswerRpcRequest>) -> ResultFuture<any_pointer::Reader<'a>, any_pointer::Pipeline> {
        let tmp = *self;
        let PromisedAnswerRpcRequest { rpc_chan, mut message, mut answer_ref, ops } = tmp;
        let (answer_tx, answer_rx) = ::std::comm::channel();

        let (interface_id, method_id) = match message.get_root::<message::Builder>().which() {
            Some(message::Call(call)) => {
                (call.get_interface_id(), call.get_method_id())
            }
            _ => {
                panic!("bad call");
            }
        };

        let context =
            (box PromisedAnswerRpcCallContext::new(message, rpc_chan.clone(), answer_tx))
            as Box<CallContextHook+Send>;

        answer_ref.receive(interface_id, method_id, ops, context);

        let pipeline = box PromisedAnswerRpcPipeline;
        let typeless = any_pointer::Pipeline::new(pipeline as Box<PipelineHook+Send>);

        ResultFuture {answer_port : answer_rx, answer_result : Err(()) /* XXX */,
                       pipeline : typeless  }
    }
}


pub struct RpcPipeline {
    channel : ::std::comm::Sender<RpcEvent>,
    question_ref : QuestionRef,
}

impl PipelineHook for RpcPipeline {
    fn copy(&self) -> Box<PipelineHook+Send> {
        (box RpcPipeline { channel : self.channel.clone(),
                        question_ref : self.question_ref.clone() }) as Box<PipelineHook+Send>
    }
    fn get_pipelined_cap(&self, ops : Vec<pipeline_op::Type>) -> Box<ClientHook+Send> {
        (box PipelineClient { channel : self.channel.clone(),
                           ops : ops,
                           question_ref : self.question_ref.clone(),
        }) as Box<ClientHook+Send>
    }
}

pub struct PromisedAnswerRpcPipeline;

impl PipelineHook for PromisedAnswerRpcPipeline {
    fn copy(&self) -> Box<PipelineHook+Send> {
        (box PromisedAnswerRpcPipeline) as Box<PipelineHook+Send>
    }
    fn get_pipelined_cap(&self, _ops : Vec<pipeline_op::Type>) -> Box<ClientHook+Send> {
        panic!()
    }
}

pub struct Aborter {
    succeeded : bool,
    answer_id : AnswerId,
    rpc_chan : ::std::comm::Sender<RpcEvent>,
}

impl Drop for Aborter {
    fn drop(&mut self) {
        if !self.succeeded {
            let mut results_message = box MallocMessageBuilder::new_default();
            {
                let root : message::Builder = results_message.init_root();
                let ret = root.init_return();
                ret.set_answer_id(self.answer_id);
                let exc = ret.init_exception();
                exc.set_reason("aborted");
            }
            self.rpc_chan.send_opt(ReturnEvent(results_message)).is_ok();
        }
    }
}

pub struct RpcCallContext {
    params_message : Box<OwnedSpaceMessageReader>,
    results_message : Box<MallocMessageBuilder>,
    rpc_chan : ::std::comm::Sender<RpcEvent>,
    aborter : Aborter,
}

impl RpcCallContext {
    pub fn new(params_message : Box<OwnedSpaceMessageReader>,
               rpc_chan : ::std::comm::Sender<RpcEvent>) -> RpcCallContext {
        let answer_id = {
            let root : message::Reader = params_message.get_root();
            match root.which() {
                Some(message::Call(call)) => {
                    call.get_question_id()
                }
                _ => panic!(),
            }
        };
        let mut results_message = box MallocMessageBuilder::new(*BuilderOptions::new().fail_fast(false));
        {
            let root : message::Builder = results_message.init_root();
            let ret = root.init_return();
            ret.set_answer_id(answer_id);
            ret.init_results();
        }
        RpcCallContext {
            params_message : params_message,
            results_message : results_message,
            rpc_chan : rpc_chan.clone(),
            aborter : Aborter { succeeded : false, answer_id : answer_id, rpc_chan : rpc_chan},
        }
    }
}

impl CallContextHook for RpcCallContext {
    fn get<'a>(&'a mut self) -> (any_pointer::Reader<'a>, any_pointer::Builder<'a>) {

        let params = {
            let root : message::Reader = self.params_message.get_root();
            match root.which() {
                Some(message::Call(call)) => {
                    call.get_params().get_content()
                }
                _ => panic!(),
            }
        };

        let results = {
            let root : message::Builder = self.results_message.get_root();
            match root.which() {
                Some(message::Return(ret)) => {
                    match ret.which() {
                        Some(return_::Results(results)) => {
                            results.get_content()
                        }
                        _ => panic!(),
                    }
                }
                _ => panic!(),
            }
        };

        (params, results)
    }
    fn fail(mut self : Box<RpcCallContext>) {
        self.aborter.succeeded = false;
    }

    fn done(self : Box<RpcCallContext>) {
        let tmp = *self;
        let RpcCallContext { params_message : _, mut results_message, rpc_chan, mut aborter} = tmp;
        aborter.succeeded = true;
        write_outgoing_cap_table(&rpc_chan, &mut *results_message);

        rpc_chan.send(ReturnEvent(results_message));
    }
}

pub struct LocalResponse {
    message : Box<MallocMessageBuilder>,
}

impl LocalResponse {
    pub fn new(message : Box<MallocMessageBuilder>) -> LocalResponse {
        LocalResponse { message : message }
    }
}

impl ResponseHook for LocalResponse {
    fn get<'a>(&'a mut self) -> any_pointer::Reader<'a> {
        self.message.get_root_internal().as_reader()
    }
}


pub struct PromisedAnswerRpcCallContext {
    params_message : Box<MallocMessageBuilder>,
    results_message : Box<MallocMessageBuilder>,
    rpc_chan : ::std::comm::Sender<RpcEvent>,
    answer_chan : ::std::comm::Sender<Box<ResponseHook+Send>>,
}

impl PromisedAnswerRpcCallContext {
    pub fn new(params_message : Box <MallocMessageBuilder>,
               rpc_chan : ::std::comm::Sender<RpcEvent>,
               answer_chan : ::std::comm::Sender<Box<ResponseHook+Send>>)
               -> PromisedAnswerRpcCallContext {


        let mut results_message = box MallocMessageBuilder::new(*BuilderOptions::new().fail_fast(false));
        {
            let root : message::Builder = results_message.init_root();
            let ret = root.init_return();
            ret.init_results();
        }
        PromisedAnswerRpcCallContext {
            params_message : params_message,
            results_message : results_message,
            rpc_chan : rpc_chan,
            answer_chan : answer_chan,
        }
    }
}

impl CallContextHook for PromisedAnswerRpcCallContext {
    fn get<'a>(&'a mut self) -> (any_pointer::Reader<'a>, any_pointer::Builder<'a>) {

        let params = {
            let root : message::Builder = self.params_message.get_root();
            match root.which() {
                Some(message::Call(call)) => {
                    call.get_params().get_content().as_reader()
                }
                _ => panic!(),
            }
        };

        let results = {
            let root : message::Builder = self.results_message.get_root();
            match root.which() {
                Some(message::Return(ret)) => {
                    match ret.which() {
                        Some(return_::Results(results)) => {
                            results.get_content()
                        }
                        _ => panic!(),
                    }
                }
                _ => panic!(),
            }
        };

        (params, results)
    }
    fn fail(self : Box<PromisedAnswerRpcCallContext>) {
        let tmp = *self;
        let PromisedAnswerRpcCallContext {
            params_message : _, mut results_message, rpc_chan : _, answer_chan} = tmp;

        match results_message.get_root::<message::Builder>().which() {
            Some(message::Return(ret)) => {
                let exc = ret.init_exception();
                exc.set_reason("aborted");
            }
            _ => panic!(),
        }

        answer_chan.send(box LocalResponse::new(results_message) as Box<ResponseHook+Send>);

    }

    fn done(self : Box<PromisedAnswerRpcCallContext>) {
        let tmp = *self;

        let PromisedAnswerRpcCallContext {
            params_message : _, results_message, rpc_chan : _, answer_chan} = tmp;

        answer_chan.send(box LocalResponse::new(results_message) as Box<ResponseHook+Send>);
    }
}


pub struct OutgoingMessage {
    message : Box<MallocMessageBuilder>,
    answer_chan : ::std::comm::Sender<Box<ResponseHook+Send>>,
    question_chan : ::std::comm::Sender<QuestionRef>,
}


pub enum RpcEvent {
    IncomingMessage(Box<serialize::OwnedSpaceMessageReader>),
    Outgoing(OutgoingMessage),
    NewLocalServer(Box<ClientHook+Send>, ::std::comm::Sender<ExportId>),
    ReturnEvent(Box<MallocMessageBuilder>),
    DoneWithQuestion(QuestionId),
    ShutdownEvent,
}


impl RpcEvent {
    pub fn new_outgoing(message : Box<MallocMessageBuilder>)
                        -> (OutgoingMessage, ::std::comm::Receiver<Box<ResponseHook+Send>>,
                            ::std::comm::Receiver<QuestionRef>) {
        let (answer_chan, answer_port) = ::std::comm::channel::<Box<ResponseHook+Send>>();

        let (question_chan, question_port) = ::std::comm::channel::<QuestionRef>();

        (OutgoingMessage{ message : message,
                          answer_chan : answer_chan,
                          question_chan : question_chan },
         answer_port,
         question_port)
    }
}

