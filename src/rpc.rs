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
               _bootstrap_interface: Option<::capnp::capability::Client>) -> System<VatId> {
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

    fn bootstrap(state: Rc<RefCell<ConnectionState<VatId>>>) -> Box<ClientHook> {
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
                let mut message = c.new_outgoing_message(100); // TODO estimate size
                {
                    let mut builder = message.get_body().unwrap().init_as::<message::Builder>().init_bootstrap();
                    builder.set_question_id(question_id);
                }
                message.send();
            }
            &mut Err(_) => panic!(),
        }

        // construct a pipeline out of `promise`
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

struct Request<VatId> where VatId: 'static {
    connection_state: Rc<RefCell<ConnectionState<VatId>>>,
    target: Client<VatId>,
    message: Box<::OutgoingMessage>,
}

impl <VatId> Request<VatId> where VatId: 'static {
    fn new(connection_state: Rc<RefCell<ConnectionState<VatId>>>,
           size_hint: Option<::capnp::MessageSize>,
           target: Client<VatId>) -> Request<VatId> {

        let message = connection_state.borrow_mut().connection.as_mut().expect("not connected?")
            .new_outgoing_message(100);
        Request {
            connection_state: connection_state,
            target: target,
            message: message,
        }
    }

    fn get_root<'a>(&'a mut self) -> any_pointer::Builder<'a> {
        // TODO imbue
        self.get_call().get_params().unwrap().get_content()
    }

    fn get_call<'a>(&'a mut self) -> ::rpc_capnp::call::Builder<'a> {
        self.message.get_body().unwrap().get_as().unwrap()
    }
}

impl <VatId> RequestHook for Request<VatId> {
    fn init<'a>(&'a mut self) -> any_pointer::Builder<'a> {
        unimplemented!()
    }
    fn send<'a>(self: Box<Self>) -> ::capnp::capability::RemotePromise<any_pointer::Owned> {
        unimplemented!()
    }
}

enum PipelineState<VatId> where VatId: 'static {
    Waiting(Rc<RefCell<QuestionRef<VatId>>>),
    Resolved(Rc<RefCell<Response<VatId>>>),
    Broken(::capnp::Error),
}

struct Pipeline<VatId> where VatId: 'static {
    connection_state: Rc<RefCell<ConnectionState<VatId>>>,
    state: PipelineState<VatId>,
    redirect_later: Option<RefCell<::gj::ForkedPromise<Rc<RefCell<Response<VatId>>>>>>,
}

impl <VatId> Pipeline<VatId> {
    fn new(connection_state: Rc<RefCell<ConnectionState<VatId>>>,
           question_ref: Rc<RefCell<QuestionRef<VatId>>>,
           redirect_later: Option<::gj::Promise<Rc<RefCell<Response<VatId>>>, Box<::std::error::Error>>>)
           -> Rc<RefCell<Pipeline<VatId>>>
    {
        match redirect_later {
            Some(redirect_later_promise) => {
                let fork = redirect_later_promise.fork();
                let result = Rc::new(RefCell::new(Pipeline {
                    connection_state: connection_state,
                    state: PipelineState::Waiting(question_ref),
                    redirect_later: None,
                }));

                let this = result.clone();
/*
                fork.add_branch().map_else(move |response| {
                    match
                    this.borrow_mut().resolve(response);
                    Ok(())
                });*/

                result.borrow_mut().redirect_later = Some(RefCell::new(fork));
                result
            }
            None =>
                Rc::new(RefCell::new(Pipeline {
                    connection_state: connection_state,
                    state: PipelineState::Waiting(question_ref),
                    redirect_later: None,
                }))
        }
    }

    fn resolve(&mut self, response: Rc<RefCell<Response<VatId>>>) {
        match self.state { PipelineState::Waiting( _ ) => (), _ => panic!("Already resolved?") }
        self.state = PipelineState::Resolved(response);
    }

    fn resolve_err(&mut self, err: ()) {
        unimplemented!()
    }
}

impl <VatId> PipelineHook for Pipeline<VatId> {
    fn add_ref(&self) -> Box<PipelineHook> {
//        self.clone();
        unimplemented!()
    }
    fn get_pipelined_cap(&self, ops: &[PipelineOp]) -> Box<ClientHook> {
        let mut copy = Vec::new();
        for &op in ops {
            copy.push(op)
        }
        self.get_pipelined_cap_move(copy)
    }
    fn get_pipelined_cap_move(&self, ops: Vec<PipelineOp>) -> Box<ClientHook> {
        match &self.state {
            &PipelineState::Waiting(ref question_ref) => {
                // Wrap a PipelineClient in a PromiseClient.
                question_ref.clone();

                match &self.redirect_later {
                    &Some(ref r) => {
                        let resolution_promise = r.borrow_mut().add_branch().map(move |response| {
                            Ok(response.borrow().get().unwrap().get_pipelined_cap(&ops))
                        });
                        // return PromiseClient.
                        unimplemented!()
                    }
                    &None => {
                        // Oh, this pipeline will never get redirected, so just return the PipelineClient.
                        unimplemented!()
                    }
                }
            }
            &PipelineState::Resolved(ref response) => {
                response.borrow_mut().get().unwrap().get_pipelined_cap(&ops[..]).unwrap()
            }
            &PipelineState::Broken(ref response) => { unimplemented!() }
        }
    }
}

enum ClientVariant<VatId> where VatId: 'static {
    Import(ImportClient<VatId>),
    Pipeline(PipelineClient<VatId>),
    Promise(PromiseClient<VatId>),
    Broken(BrokenClient),
    NoIntercept(()),
}

struct Client<VatId> where VatId: 'static {
    connection_state: Rc<RefCell<ConnectionState<VatId>>>,
    variant: ClientVariant<VatId>,
}

struct ImportClient<VatId> where VatId: 'static {
    connection_state: Rc<RefCell<ConnectionState<VatId>>>,
    import_id: ImportId,

    /// Number of times we've received this import from the peer.
    remote_ref_count: u32,
}

impl <VatId> Drop for ImportClient<VatId> {
    fn drop(&mut self) {
        // Remove self from the import table, if the table is still pointing at us.
        // ...

        // Send a message releasing our remote references.
        // ...
    }
}

impl <VatId> ImportClient<VatId> where VatId: 'static {
    fn new(connection_state: Rc<RefCell<ConnectionState<VatId>>>, import_id: ImportId) -> ImportClient<VatId> {
        ImportClient {
            connection_state: connection_state,
            import_id: import_id,
            remote_ref_count: 0,
        }
    }
}

/// A ClientHook representing a pipelined promise.  Always wrapped in PromiseClient.
struct PipelineClient<VatId> where VatId: 'static {
    connection_state: Rc<RefCell<ConnectionState<VatId>>>,
    question_ref: Rc<RefCell<QuestionRef<VatId>>>,
    ops: Vec<PipelineOp>,
}

impl <VatId> PipelineClient<VatId> where VatId: 'static {
    fn new(connection_state: Rc<RefCell<ConnectionState<VatId>>>,
           question_ref: Rc<RefCell<QuestionRef<VatId>>>,
           ops: Vec<PipelineOp>) -> PipelineClient<VatId> {
        PipelineClient {
            connection_state: connection_state,
            question_ref: question_ref,
            ops: ops,
        }
    }
}

/// A ClientHook that initially wraps one client and then, later on, redirects
/// to some other client.
struct PromiseClient<VatId> where VatId: 'static {
    connection_state: Rc<RefCell<ConnectionState<VatId>>>,
    initial: Rc<RefCell<Box<ClientHook>>>,
    eventual: ::gj::Promise<Rc<RefCell<Box<ClientHook>>>, Box<::std::error::Error>>,
    importId: Option<ImportId>,
    is_resolved: bool,
    fork: ::gj::ForkedPromise<Rc<RefCell<Box<ClientHook>>>>,
}

impl <VatId> PromiseClient<VatId> {
    fn new(connection_state: Rc<RefCell<ConnectionState<VatId>>>,
           initial: Rc<RefCell<Box<ClientHook>>>,
           eventual: ::gj::Promise<Rc<RefCell<Box<ClientHook>>>, Box<::std::error::Error>>,
           importId: Option<ImportId>) -> PromiseClient<VatId> {
        unimplemented!()
    }
}

struct BrokenClient;

impl <VatId> Client<VatId> {
    fn write_target(&self, mut target: ::rpc_capnp::message_target::Builder)
                    -> Option<Rc<RefCell<Box<ClientHook>>>>
    {
        match &self.variant {
            &ClientVariant::Import(ref import_client) => {
                target.set_imported_cap(import_client.import_id);
                None
            }
            &ClientVariant::Pipeline(ref pipeline_client) => {
                let mut builder = target.init_promised_answer();
                builder.set_question_id(pipeline_client.question_ref.borrow().id);
                // adopt_transform
                None
            }
            &ClientVariant::Promise(ref promise_client) => {
                unimplemented!()
            }
            _ => {
                unimplemented!()
            }
        }
    }
}

impl <VatId> Clone for Client<VatId> {
    fn clone(&self) -> Client<VatId> {
        unimplemented!()
    }
}

impl <VatId> ClientHook for Client<VatId> {
    fn add_ref(&self) -> Box<ClientHook> {
        Box::new(self.clone())
    }
    fn new_call(&self, interface_id: u64, method_id: u16,
                size_hint: Option<::capnp::MessageSize>)
                -> ::capnp::capability::Request<any_pointer::Owned, any_pointer::Owned>
    {
        let mut request = Request::new(self.connection_state.clone(), size_hint, self.clone());
        {
            let mut call_builder = request.get_call();
            call_builder.set_interface_id(interface_id);
            call_builder.set_method_id(method_id);
        }

        ::capnp::capability::Request::new(Box::new(request))
    }

    fn call(&self, interface_id: u64, method_id: u16, params: Box<ParamsHook>, results: Box<ResultsHook>) {
        let mut request = self.new_call(interface_id, method_id,
                                        Some(params.get().total_size().unwrap()));
        request.init().set_as(params.get()).unwrap();

        // We can and should propagate cancellation.
        // context -> allowCancellation();

        unimplemented!()
    }

    fn get_brand(&self) -> usize {
        unimplemented!()
    }

    fn get_descriptor(&self) -> Box<::std::any::Any> {
        unimplemented!()
    }
}
