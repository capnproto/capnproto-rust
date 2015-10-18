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
use capnp::capability::{RemotePromise, Request};
use capnp::private::capability::{ClientHook, ParamsHook, PipelineHook, PipelineOp,
                                 RequestHook, ResponseHook, ResultsHook};

use std::vec::Vec;
use std::collections::hash_map::HashMap;
use std::collections::binary_heap::BinaryHeap;
use std::cell::RefCell;
use std::rc::Rc;

use rpc_capnp::{message, return_, cap_descriptor, message_target, payload, promised_answer};


pub struct System<VatId> where VatId: 'static {
    network: Box<::VatNetwork<VatId>>,
    connection_state: Option<Rc<RefCell<ConnectionState<VatId>>>>,
}

impl <VatId> System <VatId> {
    pub fn new(network: Box<::VatNetwork<VatId>>,
               bootstrap_interface: ::capnp::capability::Client) -> System<VatId> {
        System { network: network, connection_state: None }
    }

    /// Connects to the given vat and return its bootstrap interface.
    pub fn bootstrap(&mut self, vat_id: VatId) -> ::capnp::capability::Client {
        let connection = match self.network.connect(vat_id) {
            Some(connection) => connection,
            None => unimplemented!(),
        };
        let mut connection_state = ConnectionState::new(connection);
        let hook = ConnectionState::bootstrap(connection_state.clone());
        self.connection_state = Some(connection_state);
        ::capnp::capability::Client::new(hook)
    }
}

pub type QuestionId = u32;
pub type AnswerId = QuestionId;
pub type ExportId = u32;
pub type ImportId = ExportId;

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

    pub fn push(&mut self, val: T) -> u32 {
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

struct Question<VatId> where VatId: 'static {
    is_awaiting_return: bool,
    param_exports: Vec<ExportId>,

    /// The local QuestionRef, set to None when it is destroyed.
    self_ref: Option<Rc<RefCell<QuestionRef<VatId>>>>
}

impl <VatId> Question<VatId> {
    fn new() -> Question<VatId> {
        Question { is_awaiting_return: true, param_exports: Vec::new(), self_ref: None }
    }
}

/// A reference to an entry on the question table.  Used to detect when the `Finish` message
/// can be sent.
struct QuestionRef<VatId> where VatId: 'static {
    connection_state: Rc<RefCell<ConnectionState<VatId>>>,
    id: QuestionId,
    fulfiller: ::gj::PromiseFulfiller<Response<VatId>, ()>,
}

impl <VatId> QuestionRef<VatId> {
    fn new(state: Rc<RefCell<ConnectionState<VatId>>>, id: QuestionId,
           fulfiller: ::gj::PromiseFulfiller<Response<VatId>, ()>) -> QuestionRef<VatId> {
        QuestionRef { connection_state: state, id: id, fulfiller: fulfiller }
    }
}

impl <VatId> Drop for QuestionRef<VatId> {
    fn drop(&mut self) {
        // TODO send the Finish message.
    }
}

struct Answer {
    active: bool,
}

pub struct Export {
    ref_count: usize
}

pub struct Import {
    import_client: (),
}

pub struct ConnectionErrorHandler<VatId> where VatId: 'static {
    state: Rc<RefCell<ConnectionState<VatId>>>,
}

impl <VatId> ::gj::TaskReaper<(), ::capnp::Error> for ConnectionErrorHandler<VatId> {
    fn task_failed(&mut self, error: ::capnp::Error) {
        self.state.borrow_mut().disconnect(error);
    }
}

struct ConnectionState<VatId> where VatId: 'static {
    exports: ExportTable<Export>,
    questions: ExportTable<Question<VatId>>,
    answers: ImportTable<Answer>,
    imports: ImportTable<Import>,
    tasks: Option<::gj::TaskSet<(), ::capnp::Error>>,
    connection: ::std::result::Result<Box<::Connection<VatId>>, ::capnp::Error>,
}

impl <VatId> ConnectionState<VatId> {
    fn new(connection: Box<::Connection<VatId>>) -> Rc<RefCell<ConnectionState<VatId>>> {
        let state = Rc::new(RefCell::new(ConnectionState {
            exports: ExportTable::new(),
            questions: ExportTable::new(),
            answers: ImportTable::new(),
            imports: ImportTable::new(),
            tasks: None,
            connection: Ok(connection)
        }));
        let mut task_set = ::gj::TaskSet::new(Box::new(ConnectionErrorHandler { state: state.clone() }));
        task_set.add(ConnectionState::message_loop(state.clone()));
        state.borrow_mut().tasks = Some(task_set);
        state
    }

    fn disconnect(&mut self, error: ::capnp::Error) {
        if self.connection.is_err() {
            // Already disconnected.
            return;
        }

        // TODO ...
    }

    fn bootstrap(state: Rc<RefCell<ConnectionState<VatId>>>) -> Rc<RefCell<Box<ClientHook>>> {
        let question_id = state.borrow_mut().questions.push(Question::new());

        let (promise, fulfiller) = ::gj::new_promise_and_fulfiller();
        let question_ref = Rc::new(RefCell::new(QuestionRef::new(state.clone(), question_id, fulfiller)));
        match &mut state.borrow_mut().questions.slots[question_id as usize] {
            &mut Some(ref mut q) => {
                q.self_ref = Some(question_ref.clone());
            }
            &mut None => unreachable!(),
        }
        match &mut state.borrow_mut().connection {
            &mut Ok(ref mut c) => {
                let mut message = c.new_outgoing_message();
                {
                    let mut builder = message.get_body().unwrap().init_as::<message::Builder>().init_bootstrap();
                    builder.set_question_id(question_id);
                }
                message.send();
            }
            &mut Err(_) => panic!(),
        }

        unimplemented!()
    }

    fn message_loop(state: Rc<RefCell<ConnectionState<VatId>>>) -> ::gj::Promise<(), ::capnp::Error> {
        let promise = match state.borrow_mut().connection {
            Err(ref e) => return ::gj::Promise::fulfilled(()),
            Ok(ref mut connection) => connection.receive_incoming_message(),
        };
        let state1 = state.clone();
        promise.map(move |message| {
            match message {
                Some(m) => {
                    ConnectionState::handle_message(state, m).map(|()| true)
                }
                None => {
                    state.borrow_mut().disconnect(
                        ::capnp::Error::Io(::std::io::Error::new(::std::io::ErrorKind::Other,
                                                                 "Peer disconnected")));
                    Ok(false)
                }
            }
        }).then(move |keepGoing| {
            if keepGoing {
                Ok(ConnectionState::message_loop(state1))
            } else {
                Ok(::gj::Promise::fulfilled(()))
            }
        })
    }

    fn handle_message(state: Rc<RefCell<ConnectionState<VatId>>>,
                      message: Box<::IncomingMessage>) -> ::capnp::Result<()> {
        let reader = try!(try!(message.get_body()).get_as::<message::Reader>());
        match try!(reader.which()) {
            message::Unimplemented(_) => {}
            message::Abort(_) => {}
            message::Bootstrap(_) => {}
            message::Call(_) => {}
            message::Return(_) => {}
            message::Finish(_) => {}
            message::Resolve(_) => {}
            message::Release(_) => {}
            message::Disembargo(_) => {}
            message::Provide(_) => {}
            message::Accept(_) => {}
            message::Join(_) => {}
            message::ObsoleteSave(_) | message::ObsoleteDelete(_) => {}
        }
        Ok(())
    }
}

struct Response<VatId> where VatId: 'static {
    connection_state: Rc<RefCell<ConnectionState<VatId>>>,
    message: Box<::IncomingMessage>,
//    cap_table:
    question_ref: Rc<RefCell<QuestionRef<VatId>>>,
}

impl <VatId> ResponseHook for Response<VatId> {
    fn get<'a>(&'a self) -> ::capnp::Result<any_pointer::Reader<'a>> {
        match try!(try!(try!(self.message.get_body()).get_as::<message::Reader>()).which()) {
            message::Return(Ok(ret)) => {
                match try!(ret.which()) {
                    return_::Results(Ok(payload)) => {
                        payload.get_cap_table(); // TODO imbue
                        Ok(payload.get_content())
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }
}

enum PipelineState<VatId> where VatId: 'static {
    Waiting(Rc<RefCell<QuestionRef<VatId>>>),
    Resolved(Response<VatId>),
    Broken(::capnp::Error),
}

struct Pipeline<VatId> where VatId: 'static {
    connection_state: Rc<RefCell<ConnectionState<VatId>>>,
    state: PipelineState<VatId>,
}

/*impl <VatId> PipelineHook<VatId> {
    fn new(state: Rc<RefCell<ConnectionState<VatId>>>, question_ref: Rc<RefCell<QuestionRef<VatId>>>,
           redirect_later_param: ::gj::Promise<
}*/

impl <VatId> PipelineHook for Pipeline<VatId> {
    fn get_pipelined_cap(&self, ops: Vec<PipelineOp>) -> Rc<RefCell<Box<ClientHook>>> {
        match &self.state {
            &PipelineState::Waiting(ref question_ref) => { unimplemented!() }
            &PipelineState::Resolved(ref response) => {
                response.get().unwrap().get_pipelined_cap(&ops[..]).unwrap()
            }
            &PipelineState::Broken(ref response) => { unimplemented!() }
        }
    }
}

enum Client<VatId> where VatId: 'static {
    Import(ImportClient<VatId>),
    Pipeline(PipelineClient<VatId>),
    Promise(PromiseClient<VatId>),
    NoIntercept(()),
}

struct ImportClient<VatId> where VatId: 'static {
    connection_state: Rc<RefCell<ConnectionState<VatId>>>,
}

struct PipelineClient<VatId> where VatId: 'static {
    connection_state: Rc<RefCell<ConnectionState<VatId>>>,
}

struct PromiseClient<VatId> where VatId: 'static {
    connection_state: Rc<RefCell<ConnectionState<VatId>>>,
}

impl <VatId> ClientHook for Client<VatId> {
    fn new_call(&self, _interface_id: u64, _method_id: u16,
                _size_hint: Option<::capnp::MessageSize>)
                -> ::capnp::capability::Request<any_pointer::Owned, any_pointer::Owned>
    {
        unimplemented!()
    }

    fn call(&self, _interface_id: u64, _method_id: u16, _params: Box<ParamsHook>, _results: Box<ResultsHook>) {
        unimplemented!()
    }

    fn get_descriptor(&self) -> Box<::std::any::Any> {
        unimplemented!()
    }
}
