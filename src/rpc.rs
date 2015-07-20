// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use capnp::{any_pointer};
use capnp::capability;
use capnp::capability::{ResultFuture, Request};
use capnp::private::capability::{CallContextHook, ClientHook, PipelineHook, PipelineOp,
                                 RequestHook, ResponseHook};
use capnp::serialize;

use std::vec::Vec;
use std::collections::hash_map::HashMap;
use std::collections::binary_heap::BinaryHeap;

use std::sync::{Arc, Mutex};

use rpc_capnp::{message, return_, cap_descriptor, message_target, payload, promised_answer};

pub type QuestionId = u32;
pub type AnswerId = QuestionId;
pub type ExportId = u32;
pub type ImportId = ExportId;

pub struct Question {
    chan : ::std::sync::mpsc::Sender<Box<ResponseHook+Send>>,
    is_awaiting_return : bool,
    ref_counter : ::std::sync::mpsc::Receiver<()>,
}

impl Question {
    pub fn new(sender : ::std::sync::mpsc::Sender<Box<ResponseHook+Send>>) -> (Question, ::std::sync::mpsc::Sender<()>) {
        let (tx, rx) = ::std::sync::mpsc::channel::<()>();
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
    ref_count : ::std::sync::mpsc::Sender<()>,

    rpc_chan : ::std::sync::mpsc::Sender<RpcEvent>,
}

impl QuestionRef {
    pub fn new(id : u32, ref_count : ::std::sync::mpsc::Sender<()>,
               rpc_chan : ::std::sync::mpsc::Sender<RpcEvent>) -> QuestionRef {
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
    Sent(Box<::capnp::message::Builder<::capnp::message::HeapAllocator>>),
    Pending(Vec<(u64, u16, Vec<PipelineOp>, Box<CallContextHook+Send>)>),
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
            status : Arc::new(Mutex::new(AnswerStatus::Pending(Vec::new()))),
        }
    }

    fn do_call(answer_message : &mut Box<::capnp::message::Builder<::capnp::message::HeapAllocator>>, interface_id : u64, method_id : u16,
               ops : Vec<PipelineOp>, context : Box<CallContextHook+Send>) {
        let root : message::Builder = answer_message.get_root().unwrap();
        match root.which() {
            Ok(message::Return(Ok(ret))) => {
                match ret.which() {
                    Ok(return_::Results(Ok(payload))) => {
                        let hook = payload.get_content().as_reader().
                            get_pipelined_cap(&ops).unwrap();
                        hook.call(interface_id, method_id, context);
                    }
                    Ok(return_::Exception(_exc)) => {
                        // TODO
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    pub fn receive(&mut self, interface_id : u64, method_id : u16,
                   ops : Vec<PipelineOp>, context : Box<CallContextHook+Send>) {
        match &mut *self.status.lock().unwrap() {
            &mut AnswerStatus::Sent(ref mut answer_message) => {
                AnswerRef::do_call(answer_message, interface_id, method_id, ops, context);
            }
            &mut AnswerStatus::Pending(ref mut waiters) => {
                waiters.push((interface_id, method_id, ops, context));
            }
        }
    }

    pub fn sent(&mut self, mut message : Box<::capnp::message::Builder<::capnp::message::HeapAllocator>>) {
        let mut lock = self.status.lock().unwrap();
        match &mut *lock {
            &mut AnswerStatus::Sent(_) => {panic!()}
            &mut AnswerStatus::Pending(ref mut waiters) => {
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
        *lock = AnswerStatus::Sent(message);
    }


}

pub struct Answer {
    answer_ref : AnswerRef,
    _result_exports : Vec<ExportId>,
}

impl Answer {
    pub fn new() -> Answer {
        Answer {
            answer_ref : AnswerRef::new(),
            _result_exports : Vec::new(),
        }
    }
}

pub struct Export {
    hook : Box<ClientHook+Send>,
    _reference_count : i32,
}

impl Export {
    pub fn new(hook : Box<ClientHook+Send>) -> Export {
        Export { hook : hook, _reference_count : 0 }
    }
}

#[derive(Clone, Copy)]
pub struct Import;

pub struct ImportTable<T> {
    slots : HashMap<u32, T>,
}

impl <T> ImportTable<T> {
    pub fn new() -> ImportTable<T> {
        ImportTable { slots : HashMap::new() }
    }
}

#[derive(PartialEq, Eq)]
struct ReverseU32 { val : u32 }

impl ::std::cmp::Ord for ReverseU32 {
    fn cmp(&self, other : &ReverseU32) -> ::std::cmp::Ordering {
        if self.val > other.val { ::std::cmp::Ordering::Less }
        else if self.val < other.val { ::std::cmp::Ordering::Greater }
        else { ::std::cmp::Ordering::Equal }
    }
}

impl ::std::cmp::PartialOrd for ReverseU32 {
    fn partial_cmp(&self, other : &ReverseU32) -> Option<::std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}


pub struct ExportTable<T> {
    slots : Vec<Option<T>>,

    // prioritize lower values
    free_ids : BinaryHeap<ReverseU32>,
}

impl <T> ExportTable<T> {
    pub fn new() -> ExportTable<T> {
        ExportTable { slots : Vec::new(),
                      free_ids : BinaryHeap::new() }
    }

    pub fn erase(&mut self, id : u32) {
        self.slots[id as usize] = None;
        self.free_ids.push(ReverseU32 { val : id } );
    }

    pub fn push(&mut self, val : T) -> u32 {
        match self.free_ids.pop() {
            Some(ReverseU32 { val : id }) => {
                self.slots[id as usize] = Some(val);
                id
            }
            None => {
                self.slots.push(Some(val));
                self.slots.len() as u32 - 1
            }
        }
    }
}

pub struct RpcConnectionState {
    exports : ExportTable<Export>,
    questions : ExportTable<Question>,
    answers : ImportTable<Answer>,
    imports : ImportTable<Import>,
}

fn client_hooks_of_payload(payload : payload::Reader,
                           rpc_chan : &::std::sync::mpsc::Sender<RpcEvent>,
                           answers : &ImportTable<Answer>) -> Vec<Option<Box<ClientHook+Send>>> {
    let mut result = Vec::new();
    for cap in payload.get_cap_table().unwrap().iter() {
        match cap.which() {
            Ok(cap_descriptor::None(())) => {
                result.push(None)
            }
            Ok(cap_descriptor::SenderHosted(id)) => {
                let tmp : Box<ClientHook+Send> =
                    Box::new(ImportClient {
                        channel : rpc_chan.clone(),
                        import_id : id});
                result.push(Some(tmp));
            }
            Ok(cap_descriptor::SenderPromise(_id)) => {
                println!("warning: SenderPromise is unimplemented");
                result.push(None);
            }
            Ok(cap_descriptor::ReceiverHosted(_id)) => {
                panic!()
            }
            Ok(cap_descriptor::ReceiverAnswer(Ok(promised_answer))) => {
                result.push(Some(
                        Box::new(PromisedAnswerClient {
                            rpc_chan : rpc_chan.clone(),
                            ops : get_pipeline_ops(promised_answer),
                            answer_ref : answers.slots[&promised_answer.get_question_id()]
                                .answer_ref.clone(),
                        })));
            }
            Ok(cap_descriptor::ThirdPartyHosted(_)) => {
                panic!()
            }
            Err(_) => { panic!("unknown cap descriptor")}
            _ => panic!(),
        }
    }
    result
}

fn populate_cap_table(message : &mut ::capnp::message::Reader<serialize::OwnedSegments>,
                      rpc_chan : &::std::sync::mpsc::Sender<RpcEvent>,
                      answers : &ImportTable<Answer>) {
    let mut the_cap_table : Vec<Option<Box<ClientHook+Send>>> = Vec::new();
    {
        let root = message.get_root::<message::Reader>().unwrap();

        match root.which() {
            Ok(message::Return(Ok(ret))) => {
                match ret.which() {
                    Ok(return_::Results(Ok(payload))) => {
                        the_cap_table = client_hooks_of_payload(payload, rpc_chan, answers);
                    }
                    Ok(return_::Exception(_e)) => {
                    }
                    _ => {}
                }

            }
            Ok(message::Call(Ok(call))) => {
               the_cap_table = client_hooks_of_payload(call.get_params().unwrap(), rpc_chan, answers);
            }
            Ok(message::Unimplemented(_)) => {
            }
            Ok(message::Abort(_exc)) => {
            }
            Err(_) => {
            }
            _ => {
            }
        }
    }
    message.init_cap_table(the_cap_table);
}

fn get_pipeline_ops(promised_answer : promised_answer::Reader) -> Vec<PipelineOp> {
    let mut result = Vec::new();
    for op in promised_answer.get_transform().unwrap().iter() {
        match op.which() {
            Ok(promised_answer::op::Noop(())) => result.push(PipelineOp::Noop),
            Ok(promised_answer::op::GetPointerField(idx)) => result.push(PipelineOp::GetPointerField(idx)),
            Err(_) => {}
        }
    }
    return result;
}

fn finish_question<W : ::std::io::Write>(questions : &mut ExportTable<Question>,
                                         outpipe : &mut W,
                                         id : u32) {
    questions.erase(id);

    let mut finish_message = Box::new(::capnp::message::Builder::new_default());
    {
        let root : message::Builder = finish_message.init_root();
        let mut finish = root.init_finish();
        finish.set_question_id(id);
        finish.set_release_result_caps(false);
    }

    serialize::write_message(outpipe, &mut *finish_message).is_ok();
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

    pub fn run<T : ::std::io::Read + Send + 'static,
               U : ::std::io::Write + Send + 'static>(
                   self, inpipe: T, outpipe: U,
                   bootstrap_interface : Box<ClientHook + Send>,
                   opts : ::capnp::message::ReaderOptions)
        -> ::std::sync::mpsc::Sender<RpcEvent> {

        let (result_rpc_chan, port) = ::std::sync::mpsc::channel::<RpcEvent>();

        let listener_chan = result_rpc_chan.clone();

        ::std::thread::spawn(move || {
                let mut r = inpipe;
                loop {
                    match serialize::read_message(&mut r, opts) {
                        Err(_e) => { listener_chan.send(RpcEvent::Shutdown).is_ok(); break; }
                        Ok(message) => {
                            listener_chan.send(RpcEvent::IncomingMessage(Box::new(message))).is_ok();
                        }
                    }
                }
            });

        let rpc_chan = result_rpc_chan.clone();

        ::std::thread::spawn(move || {
            let RpcConnectionState {mut questions, mut exports, mut answers, imports : _imports} = self;
            let mut outpipe = outpipe;
            loop {
                match port.recv().unwrap() {
                    RpcEvent::IncomingMessage(mut message) => {
                        enum MessageReceiver {
                            Nobody,
                            Question(QuestionId),
                            Export(ExportId),
                            PromisedAnswer(AnswerId, Vec<PipelineOp>),
                        }


                        populate_cap_table(&mut *message, &rpc_chan, &answers);
                        let receiver = match message.get_root::<message::Reader>().unwrap().which() {
                            Ok(message::Unimplemented(_)) => {
                                println!("unimplemented");
                                MessageReceiver::Nobody
                            }
                            Ok(message::Abort(Ok(exc))) => {
                                println!("abort: {}", exc.get_reason().unwrap());
                                MessageReceiver::Nobody
                            }
                            Ok(message::Call(Ok(call))) => {
                                match call.get_target().unwrap().which() {
                                    Ok(message_target::ImportedCap(import_id)) => {
                                        MessageReceiver::Export(import_id)
                                    }
                                    Ok(message_target::PromisedAnswer(Ok(promised_answer))) => {
                                        MessageReceiver::PromisedAnswer(
                                            promised_answer.get_question_id(),
                                            get_pipeline_ops(promised_answer))
                                    }
                                    Err(_) => {
                                        panic!("call targets something else");
                                    }
                                    _ => panic!(),
                                }
                            }

                            Ok(message::Return(Ok(ret))) => {
                                MessageReceiver::Question(ret.get_answer_id())
                            }
                            Ok(message::Finish(Ok(finish))) => {
                                answers.slots.remove(&finish.get_question_id());
                                finish.get_release_result_caps();

                                MessageReceiver::Nobody
                            }
                            Ok(message::Resolve(_resolve)) => {
                                println!("resolve");
                                MessageReceiver::Nobody
                            }
                            Ok(message::Release(Ok(rel))) => {
                                if rel.get_reference_count() == 1 {
                                    exports.erase(rel.get_id());
                                } else {
                                    println!("warning: release count = {}", rel.get_reference_count());
                                }
                                MessageReceiver::Nobody
                            }
                            Ok(message::Disembargo(_dis)) => {
                                println!("disembargo");
                                MessageReceiver::Nobody
                            }
                            Ok(message::ObsoleteSave(_save)) => {
                                MessageReceiver::Nobody
                            }
                            Ok(message::Bootstrap(Ok(restore))) => {
                                let idx = exports.push(Export::new(bootstrap_interface.copy()));

                                let answer_id = restore.get_question_id();
                                let mut message = Box::new(::capnp::message::Builder::new_default());
                                {
                                    let root : message::Builder = message.init_root();
                                    let mut ret = root.init_return();
                                    ret.set_answer_id(answer_id);
                                    let mut payload = ret.init_results();
                                    payload.borrow().init_cap_table(1);
                                    payload.borrow().get_cap_table().unwrap().get(0).set_sender_hosted(idx);
                                    payload.get_content().set_as_capability(bootstrap_interface.copy());

                                }
                                answers.slots.insert(answer_id, Answer::new());

                                serialize::write_message(&mut outpipe, &mut *message).is_ok();
                                answers.slots.get_mut(&answer_id).unwrap().answer_ref.sent(message);

                                MessageReceiver::Nobody
                            }
                            Ok(message::ObsoleteDelete(_delete)) => {
                                MessageReceiver::Nobody
                            }
                            Ok(message::Provide(_provide)) => {
                                MessageReceiver::Nobody
                            }
                            Ok(message::Accept(_accept)) => {
                                MessageReceiver::Nobody
                            }
                            Ok(message::Join(_join)) => {
                                MessageReceiver::Nobody
                            }
                            Err(_) => {
                                println!("unknown message");
                                MessageReceiver::Nobody
                            }
                            _ => panic!(),
                        };

                        fn get_call_ids(message : &::capnp::message::Reader<serialize::OwnedSegments>) -> (QuestionId, u64, u16) {
                            let root : message::Reader = message.get_root().unwrap();
                            match root.which() {
                                Ok(message::Call(Ok(call))) =>
                                    (call.get_question_id(), call.get_interface_id(), call.get_method_id()),
                                _ => panic!(),
                            }
                        }

                        match receiver {
                            MessageReceiver::Nobody => {}
                            MessageReceiver::Question(id) => {
                                let erase_it = match &mut questions.slots[id as usize] {
                                    &mut Some(ref mut q) => {
                                        q.chan.send(Box::new(RpcResponse::new(message))).is_ok();
                                        q.is_awaiting_return = false;
                                        match q.ref_counter.try_recv() {
                                            Err(::std::sync::mpsc::TryRecvError::Disconnected) => {
                                                true
                                            }
                                            _ => {false}
                                        }
                                    }
                                    &mut None => {
                                        // XXX Todo
                                        panic!()
                                    }
                                };
                                if erase_it {
                                    finish_question(&mut questions, &mut outpipe, id);
                                }
                            }
                            MessageReceiver::Export(id) => {
                                let (answer_id, interface_id, method_id) = get_call_ids(&*message);
                                let context = Box::new(RpcCallContext::new(message, rpc_chan.clone()));

                                answers.slots.insert(answer_id, Answer::new());
                                match exports.slots[id as usize] {
                                    Some(ref ex) => {
                                        ex.hook.call(interface_id, method_id, context);
                                    }
                                    None => {
                                        // XXX todo
                                        panic!()
                                    }
                                }
                            }
                            MessageReceiver::PromisedAnswer(id, ops) => {
                                let (answer_id, interface_id, method_id) = get_call_ids(&*message);
                                let context = Box::new(RpcCallContext::new(message, rpc_chan.clone()));

                                answers.slots.insert(answer_id, Answer::new());
                                answers.slots.get_mut(&id).unwrap().answer_ref
                                    .receive(interface_id, method_id, ops, context);
                            }
                        }

                    }
                    RpcEvent::Outgoing(OutgoingMessage { message : mut m,
                                               answer_chan,
                                               question_chan} ) => {
                        {
                            let root = m.get_root::<message::Builder>().unwrap();
                            // add a question to the question table
                            match root.which() {
                                Ok(message::Return(_)) => {}
                                Ok(message::Call(Ok(mut call))) => {
                                    let (question, ref_count) = Question::new(answer_chan);
                                    let id = questions.push(question);
                                    call.set_question_id(id);
                                    let qref = QuestionRef::new(id, ref_count, rpc_chan.clone());
                                    if !question_chan.send(qref).is_ok() { panic!() }
                                }
                                Ok(message::Bootstrap(Ok(mut res))) => {
                                    let (question, ref_count) = Question::new(answer_chan);
                                    let id = questions.push(question);
                                    res.set_question_id(id);
                                    let qref = QuestionRef::new(id, ref_count, rpc_chan.clone());
                                    if !question_chan.send(qref).is_ok() { panic!() }
                                }
                                _ => {
                                    panic!("NONE OF THOSE");
                                }
                            }
                        }

                        serialize::write_message(&mut outpipe, &mut *m).is_ok();
                    }
                    RpcEvent::NewLocalServer(clienthook, export_chan) => {
                        let export_id = exports.push(Export::new(clienthook));
                        export_chan.send(export_id).unwrap();
                    }
                    RpcEvent::DoneWithQuestion(id) => {

                        // This isn't used anywhere yet.
                        // The idea is that when the last reference to a question
                        // is erased, this event will be triggered.

                        let erase_it = match questions.slots[id as usize] {
                            Some(ref q) if q.is_awaiting_return => {
                                true
                            }
                            _ => {false}
                        };
                        if erase_it {
                            finish_question(&mut questions, &mut outpipe, id);
                        }
                    }
                    RpcEvent::Return(mut message) => {
                        serialize::write_message(&mut outpipe, &mut *message).is_ok();

                        let answer_id_opt =
                            match message.get_root::<message::Builder>().unwrap().which() {
                                Ok(message::Return(Ok(ret))) => {
                                    Some(ret.get_answer_id())
                                }
                                _ => {None}
                            };

                        match answer_id_opt {
                            Some(answer_id) => {
                                answers.slots.get_mut(&answer_id).unwrap().answer_ref.sent(message)
                            }
                            _ => {}
                        }
                    }
                    RpcEvent::Shutdown => {
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
    ReceiverAnswer(QuestionId, Vec<PipelineOp>),
}

pub struct ImportClient {
    channel : ::std::sync::mpsc::Sender<RpcEvent>,
    pub import_id : ImportId,
}

impl ClientHook for ImportClient {
    fn copy(&self) -> Box<ClientHook+Send> {
        Box::new(ImportClient {channel : self.channel.clone(),
                               import_id : self.import_id})
    }

    fn new_call(&self, interface_id : u64, method_id : u16,
                _size_hint : Option<::capnp::MessageSize>)
                -> capability::Request<any_pointer::Builder, any_pointer::Reader, any_pointer::Pipeline> {
        let mut message = Box::new(::capnp::message::Builder::new_default());
        {
            let root : message::Builder = message.get_root().unwrap();
            let mut call = root.init_call();
            call.set_interface_id(interface_id);
            call.set_method_id(method_id);
            let mut target = call.init_target();
            target.set_imported_cap(self.import_id);
        }
        let hook = Box::new(RpcRequest { channel : self.channel.clone(),
                                         message : message,
                                         question_ref : None});
        Request::new(hook)
    }

    fn call(&self, _interface_id : u64, _method_id : u16, _context : Box<CallContextHook+Send>) {
        panic!()
    }

    fn get_descriptor(&self) -> Box<::std::any::Any> {
        Box::new(OwnedCapDescriptor::ReceiverHosted(self.import_id))
    }
}

pub struct PipelineClient {
    channel : ::std::sync::mpsc::Sender<RpcEvent>,
    pub ops : Vec<PipelineOp>,
    pub question_ref : QuestionRef,
}

impl ClientHook for PipelineClient {
    fn copy(&self) -> Box<ClientHook+Send> {
        Box::new(PipelineClient { channel : self.channel.clone(),
                                  ops : self.ops.clone(),
                                  question_ref : self.question_ref.clone(),
        })
    }

    fn new_call(&self, interface_id : u64, method_id : u16,
                _size_hint : Option<::capnp::MessageSize>)
                -> capability::Request<any_pointer::Builder, any_pointer::Reader, any_pointer::Pipeline> {
        let mut message = Box::new(::capnp::message::Builder::new_default());
        {
            let root : message::Builder = message.get_root().unwrap();
            let mut call = root.init_call();
            call.set_interface_id(interface_id);
            call.set_method_id(method_id);
            let target = call.init_target();
            let mut promised_answer = target.init_promised_answer();
            promised_answer.set_question_id(self.question_ref.id);
            let mut transform = promised_answer.init_transform(self.ops.len() as u32);
            for ii in 0..self.ops.len() {
                match self.ops[ii] {
                    PipelineOp::Noop => transform.borrow().get(ii as u32).set_noop(()),
                    PipelineOp::GetPointerField(idx) => transform.borrow().get(ii as u32).set_get_pointer_field(idx),
                }
            }
        }
        let hook = Box::new(RpcRequest { channel : self.channel.clone(),
                                         message : message,
                                         question_ref : Some(self.question_ref.clone())});
        Request::new(hook)
    }

    fn call(&self, _interface_id : u64, _method_id : u16, _context : Box<CallContextHook+Send>) {
        panic!()
    }

    fn get_descriptor(&self) -> Box<::std::any::Any> {
        Box::new(OwnedCapDescriptor::ReceiverAnswer(self.question_ref.id, self.ops.clone()))
    }
}

pub struct PromisedAnswerClient {
    rpc_chan : ::std::sync::mpsc::Sender<RpcEvent>,
    ops : Vec<PipelineOp>,
    answer_ref : AnswerRef,
}

impl ClientHook for PromisedAnswerClient {
    fn copy(&self) -> Box<ClientHook+Send> {
        Box::new(PromisedAnswerClient { rpc_chan : self.rpc_chan.clone(),
                                   ops : self.ops.clone(),
                                   answer_ref : self.answer_ref.clone(),
        })
    }

    fn new_call(&self, interface_id : u64, method_id : u16,
                _size_hint : Option<::capnp::MessageSize>)
                -> capability::Request<any_pointer::Builder, any_pointer::Reader, any_pointer::Pipeline> {
        let mut message = Box::new(::capnp::message::Builder::new_default());
        {
            let root : message::Builder = message.get_root().unwrap();
            let mut call = root.init_call();
            call.set_interface_id(interface_id);
            call.set_method_id(method_id);
        }

        let hook = Box::new(PromisedAnswerRpcRequest { rpc_chan : self.rpc_chan.clone(),
                                                       message : message,
                                                       answer_ref : self.answer_ref.clone(),
                                                       ops : self.ops.clone() });
        Request::new(hook)
    }

    fn call(&self, _interface_id : u64, _method_id : u16, _context : Box<CallContextHook+Send>) {
        panic!()
    }

    fn get_descriptor(&self) -> Box<::std::any::Any> {
        panic!()
    }
}


fn write_outgoing_cap_table(rpc_chan : &::std::sync::mpsc::Sender<RpcEvent>, message : &mut ::capnp::message::Builder<::capnp::message::HeapAllocator>) {
    fn write_payload(rpc_chan : &::std::sync::mpsc::Sender<RpcEvent>, cap_table : & [Box<::std::any::Any>],
                     payload : payload::Builder) {
        let mut new_cap_table = payload.init_cap_table(cap_table.len() as u32);
        for ii in 0..(cap_table.len() as u32) {
            match cap_table[ii as usize].downcast_ref::<OwnedCapDescriptor>() {
                Some(&OwnedCapDescriptor::NoDescriptor) => {}
                Some(&OwnedCapDescriptor::ReceiverHosted(import_id)) => {
                    new_cap_table.borrow().get(ii).set_receiver_hosted(import_id);
                }
                Some(&OwnedCapDescriptor::ReceiverAnswer(question_id,ref ops)) => {
                    let mut promised_answer = new_cap_table.borrow().get(ii).init_receiver_answer();
                    promised_answer.set_question_id(question_id);
                    let mut transform = promised_answer.init_transform(ops.len() as u32);
                    for jj in 0..ops.len() {
                        match ops[jj] {
                            PipelineOp::Noop => transform.borrow().get(jj as u32).set_noop(()),
                            PipelineOp::GetPointerField(idx) => transform.borrow().get(jj as u32).set_get_pointer_field(idx),
                        }
                    }
                }
                Some(&OwnedCapDescriptor::SenderHosted(export_id)) => {
                    new_cap_table.borrow().get(ii).set_sender_hosted(export_id);
                }
                None => {
                    match cap_table[ii as usize].downcast_ref::<Box<ClientHook+Send>>() {
                        Some(clienthook) => {
                            let (chan, port) = ::std::sync::mpsc::channel::<ExportId>();
                            rpc_chan.send(RpcEvent::NewLocalServer(clienthook.copy(), chan)).unwrap();
                            let idx = port.recv().unwrap();
                            new_cap_table.borrow().get(ii).set_sender_hosted(idx);
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
    let root : message::Builder = message.get_root().unwrap();
    match root.which() {
        Ok(message::Call(Ok(call))) => {
            write_payload(rpc_chan, &cap_table, call.get_params().unwrap())
        }
        Ok(message::Return(Ok(ret))) => {
            match ret.which() {
                Ok(return_::Results(Ok(payload))) => {
                    write_payload(rpc_chan, &cap_table, payload);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

pub struct RpcResponse {
    message: Box<::capnp::message::Reader<serialize::OwnedSegments>>,
}

impl RpcResponse {
    pub fn new(message : Box<::capnp::message::Reader<serialize::OwnedSegments>>) -> RpcResponse {
        RpcResponse { message : message }
    }
}

impl ResponseHook for RpcResponse {
    fn get<'a>(&'a mut self) -> any_pointer::Reader<'a> {
        self.message.get_root().unwrap()
    }
}

pub struct RpcRequest {
    channel : ::std::sync::mpsc::Sender<RpcEvent>,
    message : Box<::capnp::message::Builder<::capnp::message::HeapAllocator>>,
    question_ref : Option<QuestionRef>,
}

impl RequestHook for RpcRequest {
    fn message<'a>(&'a mut self) -> &'a mut ::capnp::message::Builder<::capnp::message::HeapAllocator> {
        &mut *self.message
    }
    fn send<'a>(self : Box<RpcRequest>) -> ResultFuture<any_pointer::Reader<'a>, any_pointer::Pipeline> {
        let tmp = *self;
        let RpcRequest { channel, mut message, question_ref : _ } = tmp;
        write_outgoing_cap_table(&channel, &mut *message);

        let (outgoing, answer_port, question_port) = RpcEvent::new_outgoing(message);
        channel.send(RpcEvent::Outgoing(outgoing)).unwrap();

        let question_ref = question_port.recv().unwrap();

        let pipeline = Box::new(RpcPipeline {channel : channel, question_ref : question_ref});
        let typeless = any_pointer::Pipeline::new(pipeline);

        ResultFuture {answer_port : answer_port, answer_result : Err(()) /* XXX */,
                       pipeline : typeless, marker : ::std::marker::PhantomData  }
    }
}

pub struct PromisedAnswerRpcRequest {
    rpc_chan : ::std::sync::mpsc::Sender<RpcEvent>,
    message : Box<::capnp::message::Builder<::capnp::message::HeapAllocator>>,
    answer_ref : AnswerRef,
    ops : Vec<PipelineOp>,
}

impl RequestHook for PromisedAnswerRpcRequest {
    fn message<'a>(&'a mut self) -> &'a mut ::capnp::message::Builder<::capnp::message::HeapAllocator> {
        &mut *self.message
    }
    fn send<'a>(self : Box<PromisedAnswerRpcRequest>) -> ResultFuture<any_pointer::Reader<'a>, any_pointer::Pipeline> {
        let tmp = *self;
        let PromisedAnswerRpcRequest { rpc_chan, mut message, mut answer_ref, ops } = tmp;
        let (answer_tx, answer_rx) = ::std::sync::mpsc::channel();

        let (interface_id, method_id) = match message.get_root::<message::Builder>().unwrap().which() {
            Ok(message::Call(Ok(mut call))) => {
                (call.borrow().get_interface_id(), call.borrow().get_method_id())
            }
            _ => {
                panic!("bad call");
            }
        };

        let context : Box<CallContextHook+Send> =
            Box::new(PromisedAnswerRpcCallContext::new(message, rpc_chan.clone(), answer_tx));

        answer_ref.receive(interface_id, method_id, ops, context);

        let pipeline = Box::new(PromisedAnswerRpcPipeline);
        let typeless = any_pointer::Pipeline::new(pipeline);

        ResultFuture {answer_port : answer_rx, answer_result : Err(()) /* XXX */,
                       pipeline : typeless, marker : ::std::marker::PhantomData  }
    }
}


pub struct RpcPipeline {
    channel : ::std::sync::mpsc::Sender<RpcEvent>,
    question_ref : QuestionRef,
}

impl PipelineHook for RpcPipeline {
    fn copy(&self) -> Box<PipelineHook+Send> {
        Box::new(RpcPipeline { channel : self.channel.clone(),
                               question_ref : self.question_ref.clone() })
    }
    fn get_pipelined_cap(&self, ops : Vec<PipelineOp>) -> Box<ClientHook+Send> {
        Box::new(PipelineClient { channel : self.channel.clone(),
                           ops : ops,
                           question_ref : self.question_ref.clone(),
        })
    }
}

#[derive(Clone, Copy)]
pub struct PromisedAnswerRpcPipeline;

impl PipelineHook for PromisedAnswerRpcPipeline {
    fn copy(&self) -> Box<PipelineHook+Send> {
        Box::new(PromisedAnswerRpcPipeline)
    }
    fn get_pipelined_cap(&self, _ops : Vec<PipelineOp>) -> Box<ClientHook+Send> {
        panic!()
    }
}

pub struct Aborter {
    succeeded : bool,
    message : String,
    answer_id : AnswerId,
    rpc_chan : ::std::sync::mpsc::Sender<RpcEvent>,
}

impl Drop for Aborter {
    fn drop(&mut self) {
        if !self.succeeded {
            let mut results_message = Box::new(::capnp::message::Builder::new_default());
            {
                let root : message::Builder = results_message.init_root();
                let mut ret = root.init_return();
                ret.set_answer_id(self.answer_id);
                let mut exc = ret.init_exception();
                exc.set_reason(&self.message[..]);
            }
            self.rpc_chan.send(RpcEvent::Return(results_message)).is_ok();
        }
    }
}

pub struct RpcCallContext {
    params_message : Box<::capnp::message::Reader<serialize::OwnedSegments>>,
    results_message : Box<::capnp::message::Builder<::capnp::message::HeapAllocator>>,
    rpc_chan : ::std::sync::mpsc::Sender<RpcEvent>,
    aborter : Aborter,
}

impl RpcCallContext {
    pub fn new(params_message : Box<::capnp::message::Reader<serialize::OwnedSegments>>,
               rpc_chan : ::std::sync::mpsc::Sender<RpcEvent>) -> RpcCallContext {
        let answer_id = {
            let root : message::Reader = params_message.get_root().unwrap();
            match root.which() {
                Ok(message::Call(Ok(call))) => {
                    call.get_question_id()
                }
                _ => panic!(),
            }
        };
        let mut results_message = Box::new(::capnp::message::Builder::new_default());
        {
            let root : message::Builder = results_message.init_root();
            let mut ret = root.init_return();
            ret.set_answer_id(answer_id);
            ret.init_results();
        }
        RpcCallContext {
            params_message : params_message,
            results_message : results_message,
            rpc_chan : rpc_chan.clone(),
            aborter : Aborter { succeeded : false, message : "aborted".to_string(),
                                answer_id : answer_id, rpc_chan : rpc_chan},
        }
    }
}

impl CallContextHook for RpcCallContext {
    fn get<'a>(&'a mut self) -> (any_pointer::Reader<'a>, any_pointer::Builder<'a>) {

        let params = {
            let root : message::Reader = self.params_message.get_root().unwrap();
            match root.which() {
                Ok(message::Call(Ok(call))) => {
                    call.get_params().unwrap().get_content()
                }
                _ => panic!(),
            }
        };

        let results = {
            let root : message::Builder = self.results_message.get_root().unwrap();
            match root.which() {
                Ok(message::Return(Ok(ret))) => {
                    match ret.which() {
                        Ok(return_::Results(Ok(results))) => {
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
    fn fail(mut self : Box<RpcCallContext>, message: String) {
        self.aborter.succeeded = false;
        self.aborter.message = message;
    }

    fn done(self : Box<RpcCallContext>) {
        let tmp = *self;
        let RpcCallContext { params_message : _, mut results_message, rpc_chan, mut aborter} = tmp;
        aborter.succeeded = true;
        write_outgoing_cap_table(&rpc_chan, &mut *results_message);

        rpc_chan.send(RpcEvent::Return(results_message)).unwrap();
    }
}

pub struct LocalResponse {
    message : Box<::capnp::message::Builder<::capnp::message::HeapAllocator>>,
}

impl LocalResponse {
    pub fn new(message : Box<::capnp::message::Builder<::capnp::message::HeapAllocator>>) -> LocalResponse {
        LocalResponse { message : message }
    }
}

impl ResponseHook for LocalResponse {
    fn get<'a>(&'a mut self) -> any_pointer::Reader<'a> {
        self.message.get_root::<any_pointer::Builder>().unwrap().as_reader()
    }
}


pub struct PromisedAnswerRpcCallContext {
    params_message : Box<::capnp::message::Builder<::capnp::message::HeapAllocator>>,
    results_message : Box<::capnp::message::Builder<::capnp::message::HeapAllocator>>,
    rpc_chan : ::std::sync::mpsc::Sender<RpcEvent>,
    answer_chan : ::std::sync::mpsc::Sender<Box<ResponseHook+Send>>,
}

impl PromisedAnswerRpcCallContext {
    pub fn new(params_message : Box <::capnp::message::Builder<::capnp::message::HeapAllocator>>,
               rpc_chan : ::std::sync::mpsc::Sender<RpcEvent>,
               answer_chan : ::std::sync::mpsc::Sender<Box<ResponseHook+Send>>)
               -> PromisedAnswerRpcCallContext {


        let mut results_message = Box::new(::capnp::message::Builder::new_default());
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
            let root : message::Builder = self.params_message.get_root().unwrap();
            match root.which() {
                Ok(message::Call(Ok(call))) => {
                    call.get_params().unwrap().get_content().as_reader()
                }
                _ => panic!(),
            }
        };

        let results = {
            let root : message::Builder = self.results_message.get_root().unwrap();
            match root.which() {
                Ok(message::Return(Ok(ret))) => {
                    match ret.which() {
                        Ok(return_::Results(Ok(results))) => {
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
    fn fail(self : Box<PromisedAnswerRpcCallContext>, message : String) {
        let tmp = *self;
        let PromisedAnswerRpcCallContext {
            params_message : _, mut results_message, rpc_chan : _, answer_chan} = tmp;

        match results_message.get_root::<message::Builder>().unwrap().which() {
            Ok(message::Return(Ok(ret))) => {
                let mut exc = ret.init_exception();
                exc.set_reason(&message[..]);
            }
            _ => panic!(),
        }

        answer_chan.send(Box::new(LocalResponse::new(results_message))).unwrap();

    }

    fn done(self : Box<PromisedAnswerRpcCallContext>) {
        let tmp = *self;

        let PromisedAnswerRpcCallContext {
            params_message : _, results_message, rpc_chan : _, answer_chan} = tmp;

        answer_chan.send(Box::new(LocalResponse::new(results_message))).unwrap();
    }
}


pub struct OutgoingMessage {
    message : Box<::capnp::message::Builder<::capnp::message::HeapAllocator>>,
    answer_chan : ::std::sync::mpsc::Sender<Box<ResponseHook+Send>>,
    question_chan : ::std::sync::mpsc::SyncSender<QuestionRef>,
}


pub enum RpcEvent {
    IncomingMessage(Box<::capnp::message::Reader<serialize::OwnedSegments>>),
    Outgoing(OutgoingMessage),
    NewLocalServer(Box<ClientHook+Send>, ::std::sync::mpsc::Sender<ExportId>),
    Return(Box<::capnp::message::Builder<::capnp::message::HeapAllocator>>),
    DoneWithQuestion(QuestionId),
    Shutdown,
}


impl RpcEvent {
    pub fn new_outgoing(message : Box<::capnp::message::Builder<::capnp::message::HeapAllocator>>)
                        -> (OutgoingMessage, ::std::sync::mpsc::Receiver<Box<ResponseHook+Send>>,
                            ::std::sync::mpsc::Receiver<QuestionRef>) {
        let (answer_chan, answer_port) = ::std::sync::mpsc::channel::<Box<ResponseHook+Send>>();

        let (question_chan, question_port) = ::std::sync::mpsc::sync_channel::<QuestionRef>(1);

        (OutgoingMessage{ message : message,
                          answer_chan : answer_chan,
                          question_chan : question_chan },
         answer_port,
         question_port)
    }
}

