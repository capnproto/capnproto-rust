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
use capnp::Error;
use capnp::private::capability::{ClientHook, ParamsHook, PipelineHook, PipelineOp,
                                 RequestHook, ResponseHook, ResultsHook};

use std::vec::Vec;
use std::collections::hash_map::HashMap;
use std::collections::binary_heap::BinaryHeap;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use rpc_capnp::{message, return_, cap_descriptor};


pub struct System<VatId> where VatId: 'static {
    network: Box<::VatNetwork<VatId>>,
    connection_state: Option<Rc<ConnectionState<VatId>>>,
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
        let connection_state = ConnectionState::new(connection);
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
    is_tail_call: bool,

    /// The local QuestionRef, set to None when it is destroyed.
    self_ref: Option<Rc<RefCell<QuestionRef<VatId>>>>
}

impl <VatId> Question<VatId> {
    fn new() -> Question<VatId> {
        Question { is_awaiting_return: true, param_exports: Vec::new(),
                   is_tail_call: false, self_ref: None }
    }
}

/// A reference to an entry on the question table.  Used to detect when the `Finish` message
/// can be sent.
struct QuestionRef<VatId> where VatId: 'static {
    //_connection_state: Rc<ConnectionState<VatId>>,
    id: QuestionId,
    fulfiller: Option<::gj::PromiseFulfiller<Response<VatId>, Error>>,
}

impl <VatId> QuestionRef<VatId> {
    fn new(_state: Rc<ConnectionState<VatId>>, id: QuestionId,
           fulfiller: ::gj::PromiseFulfiller<Response<VatId>, Error>) -> QuestionRef<VatId> {
        QuestionRef { /*_connection_state: state,*/ id: id, fulfiller: Some(fulfiller) }
    }
    fn fulfill(&mut self, response: Response<VatId>) {
        let fulfiller = ::std::mem::replace(&mut self.fulfiller, None);
        fulfiller.expect("no fulfiller?").fulfill(response);
    }
    fn reject(&mut self, err: Error) {
        let fulfiller = ::std::mem::replace(&mut self.fulfiller, None);
        fulfiller.expect("no fulfiller?").reject(err);
    }
}

impl <VatId> Drop for QuestionRef<VatId> {
    fn drop(&mut self) {
        // TODO send the Finish message.
    }
}

struct Answer {
    _active: bool,
}

pub struct Export {
    _ref_count: usize
}

pub struct Import<VatId> where VatId: 'static {
    // Becomes null when the import is destroyed.
    import_client: Option<Rc<RefCell<ImportClient<VatId>>>>,

    // Either a copy of importClient, or, in the case of promises, the wrapping PromiseClient.
    // Becomes null when it is discarded *or* when the import is destroyed (e.g. the promise is
    // resolved and the import is no longer needed).
    app_client: Option<Client<VatId>>,

    // If non-null, the import is a promise.
    _promise_fulfiller: Option<::gj::PromiseFulfiller<Box<ClientHook>, ()>>,
}

impl <VatId> Import<VatId> {
    fn new() -> Import<VatId> {
        Import {
            import_client: None,
            app_client: None,
            _promise_fulfiller: None,
        }
    }
}

fn remote_exception_to_error(exception: ::rpc_capnp::exception::Reader) -> Error {
    let (kind, reason) = match (exception.get_type(), exception.get_reason()) {
        (Ok(::rpc_capnp::exception::Type::Failed), Ok(reason)) =>
            (::capnp::ErrorKind::Failed, reason),
        (Ok(::rpc_capnp::exception::Type::Overloaded), Ok(reason)) =>
            (::capnp::ErrorKind::Overloaded, reason),
        (Ok(::rpc_capnp::exception::Type::Disconnected), Ok(reason)) =>
            (::capnp::ErrorKind::Disconnected, reason),
        (Ok(::rpc_capnp::exception::Type::Unimplemented), Ok(reason)) =>
            (::capnp::ErrorKind::Unimplemented, reason),
        _ => (::capnp::ErrorKind::Failed, "(malformed error)"),
    };
    Error { reason: format!("remote exception: {}", reason), kind: kind }
}

pub struct ConnectionErrorHandler<VatId> where VatId: 'static {
    weak_state: ::std::rc::Weak<ConnectionState<VatId>>,
}

impl <VatId> ConnectionErrorHandler<VatId> {
    fn new(weak_state: ::std::rc::Weak<ConnectionState<VatId>>) -> ConnectionErrorHandler<VatId> {
        ConnectionErrorHandler { weak_state: weak_state }
    }
}

impl <VatId> ::gj::TaskReaper<(), ::capnp::Error> for ConnectionErrorHandler<VatId> {
    fn task_failed(&mut self, error: ::capnp::Error) {
        match self.weak_state.upgrade() {
            Some(state) => state.disconnect(error),
            None => {}
        }
    }
}

struct ConnectionState<VatId> where VatId: 'static {
    _exports: RefCell<ExportTable<Export>>,
    questions: RefCell<ExportTable<Question<VatId>>>,
    _answers: RefCell<ImportTable<Answer>>,
    imports: RefCell<ImportTable<Import<VatId>>>,
    tasks: RefCell<Option<::gj::TaskSet<(), ::capnp::Error>>>,
    connection: RefCell<::std::result::Result<Box<::Connection<VatId>>, ::capnp::Error>>,
}

impl <VatId> ConnectionState<VatId> {
    fn new(connection: Box<::Connection<VatId>>) -> Rc<ConnectionState<VatId>> {
        let state = Rc::new(ConnectionState {
            _exports: RefCell::new(ExportTable::new()),
            questions: RefCell::new(ExportTable::new()),
            _answers: RefCell::new(ImportTable::new()),
            imports: RefCell::new(ImportTable::new()),
            tasks: RefCell::new(None),
            connection: RefCell::new(Ok(connection))
        });
        let mut task_set = ::gj::TaskSet::new(Box::new(ConnectionErrorHandler::new(Rc::downgrade(&state))));
        task_set.add(ConnectionState::message_loop(Rc::downgrade(&state)));
        *state.tasks.borrow_mut() = Some(task_set);
        state
    }

    fn disconnect(&self, _error: ::capnp::Error) {
        if self.connection.borrow().is_err() {
            // Already disconnected.
            return;
        }

        // TODO ...
    }

    fn bootstrap(state: Rc<ConnectionState<VatId>>) -> Box<ClientHook> {
        let question_id = state.questions.borrow_mut().push(Question::new());

        let (promise, fulfiller) = ::gj::new_promise_and_fulfiller();
        let question_ref = Rc::new(RefCell::new(QuestionRef::new(state.clone(), question_id, fulfiller)));
        match &mut state.questions.borrow_mut().slots[question_id as usize] {
            &mut Some(ref mut q) => {
                q.self_ref = Some(question_ref.clone());
            }
            &mut None => unreachable!(),
        }
        match &mut *state.connection.borrow_mut() {
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

        let pipeline = Pipeline::new(state, question_ref, Some(promise));
        let result = pipeline.get_pipelined_cap_move(Vec::new());
        result
    }

    fn message_loop(weak_state: ::std::rc::Weak<ConnectionState<VatId>>) -> ::gj::Promise<(), ::capnp::Error> {
        let state = weak_state.upgrade().expect("dangling reference to connection state");
        let promise = match &mut *state.connection.borrow_mut() {
            &mut Err(_) => return ::gj::Promise::ok(()),
            &mut Ok(ref mut connection) => connection.receive_incoming_message(),
        };
        let weak_state1 = weak_state.clone();
        promise.map(move |message| {
            match message {
                Some(m) => {
                    ConnectionState::handle_message(weak_state, m).map(|()| true)
                }
                None => {
                    /* XXX need to do this without forming a reference cycle.
                    state.disconnect(
                        ::capnp::Error::Io(::std::io::Error::new(::std::io::ErrorKind::Other,
                                                                 "Peer disconnected"))); */
                    Ok(false)
                }
            }
        }).then(move |keep_going| {
            if keep_going {
                ConnectionState::message_loop(weak_state1)
            } else {
                ::gj::Promise::ok(())
            }
        })
    }

    fn handle_message(weak_state: ::std::rc::Weak<ConnectionState<VatId>>,
                      message: Box<::IncomingMessage>) -> ::capnp::Result<()> {

        // Someday Rust will have non-lexical borrows and this thing won't be needed.
        enum BorrowWorkaround<VatId> where VatId: 'static {
            ReturnResults(Rc<RefCell<QuestionRef<VatId>>>, Vec<Option<Box<ClientHook>>>),
            Done,
        }

        let connection_state = weak_state.upgrade().expect("dangling reference to connection state");
        let connection_state1 = connection_state.clone();
        let intermediate = {
            let reader = try!(try!(message.get_body()).get_as::<message::Reader>());
            match try!(reader.which()) {
                message::Unimplemented(_) => {
                    unimplemented!()
                }
                message::Abort(abort) => {
                    let _exc = try!(abort);
                    //println!("ABORT {}", try!(exc.get_reason()));
                    unimplemented!()
                }
                message::Bootstrap(_) => {
                    unimplemented!()
                }
                message::Call(_) => {
                    unimplemented!()
                }
                message::Return(oret) => {
                    let ret = try!(oret);
                    let question_id = ret.get_answer_id();
                    match &mut connection_state.questions.borrow_mut().slots[question_id as usize] {
                        &mut Some(ref mut question) => {
                            question.is_awaiting_return = false;
                            match &question.self_ref {
                                &Some(ref question_ref) => {
                                    match try!(ret.which()) {
                                        return_::Results(results) => {
                                            let cap_table =
                                                ConnectionState::receive_caps(connection_state1,
                                                                              try!(try!(results).get_cap_table()));
                                            BorrowWorkaround::ReturnResults(question_ref.clone(),
                                                                            try!(cap_table))
                                        }
                                        return_::Exception(e) => {
                                            question_ref.borrow_mut().reject(
                                                remote_exception_to_error(try!(e)));
                                            BorrowWorkaround::Done
                                        }
                                        return_::Canceled(_) => {
                                            unimplemented!()
                                        }
                                        return_::ResultsSentElsewhere(_) => {
                                            unimplemented!()
                                        }
                                        return_::TakeFromOtherQuestion(_) => {
                                            unimplemented!()
                                        }
                                        return_::AcceptFromThirdParty(_) => {
                                            unimplemented!()
                                        }
                                    }
                                }
                                &None => {
                                    unimplemented!()
                                }
                            }
                        }
                        &mut None => {
                            // invalid question ID
                            unimplemented!()
                        }
                    }
                }
                message::Finish(_) => {
                    unimplemented!()
                }
                message::Resolve(_) => {
                    unimplemented!()
                }
                message::Release(_) => {
                    unimplemented!()
                }
                message::Disembargo(_) => {
                    unimplemented!()
                }
                message::Provide(_) => {
                    unimplemented!()
                }
                message::Accept(_) => {
                    unimplemented!()
                }
                message::Join(_) => {
                    unimplemented!()
                }
                message::ObsoleteSave(_) | message::ObsoleteDelete(_) => {
                    unimplemented!()
                }
            }
        };
        match intermediate {
            BorrowWorkaround::ReturnResults(question_ref, cap_table) => {
                let response = Response::new(connection_state, question_ref.clone(), message, cap_table);
                question_ref.borrow_mut().fulfill(response);
            }
            BorrowWorkaround::Done => {}
        }
        Ok(())
    }

    fn get_brand(&self) -> usize {
        self as * const _ as usize
    }

    /// If calls to the given capability should pass over this connection, fill in `target`
    /// appropriately for such a call and return nullptr.  Otherwise, return a `ClientHook` to which
    /// the call should be forwarded; the caller should then delegate the call to that `ClientHook`.
    ///
    /// The main case where this ends up returning non-null is if `cap` is a promise that has
    /// recently resolved.  The application might have started building a request before the promise
    /// resolved, and so the request may have been built on the assumption that it would be sent over
    /// this network connection, but then the promise resolved to point somewhere else before the
    /// request was sent.  Now the request has to be redirected to the new target instead.
    fn write_target(&self, cap: &ClientHook, mut target: ::rpc_capnp::message_target::Builder)
        -> ::capnp::Result<Option<Box<ClientHook>>>
    {
        if cap.get_brand() == self.get_brand() {
            // Orphans would let us avoid the need for this copying..
            let mut message = ::capnp::message::Builder::new_default();
            let mut root: any_pointer::Builder = message.init_root();
            let result = cap.write_target(root.borrow());
            let mt: ::rpc_capnp::message_target::Builder = try!(root.get_as());

            // Yuck.
            match try!(mt.which()) {
                ::rpc_capnp::message_target::ImportedCap(imported_cap) => {
                    target.set_imported_cap(imported_cap);
                }
                ::rpc_capnp::message_target::PromisedAnswer(promised_answer) => {
                    try!(target.set_promised_answer(try!(promised_answer).as_reader()));
                }
            }
            Ok(result)
        } else {
            unimplemented!()
        }
    }

    fn import(state: Rc<ConnectionState<VatId>>,
              import_id: ImportId, is_promise: bool) -> Box<ClientHook> {
        let connection_state = state.clone();

        let import_client = {
            let mut slots = &mut state.imports.borrow_mut().slots;
            let mut v = slots.entry(import_id).or_insert(Import::new());
            if v.import_client.is_some() {
                v.import_client.as_ref().unwrap().clone()
            } else {
                let import_client = ImportClient::new(&connection_state, import_id);
                v.import_client = Some(import_client.clone());
                import_client
            }
        };

        // We just received a copy of this import ID, so the remote refcount has gone up.
        import_client.borrow_mut().add_remote_ref();

        if is_promise {
            unimplemented!()
        } else {
            let client: Box<Client<VatId>> = Box::new(import_client.into());
            match state.imports.borrow_mut().slots.get_mut(&import_id) {
                Some(ref mut v) => {
                    v.app_client = Some(*client.clone());
                }
                None => { unreachable!() }
            };

            client
        }
    }

    fn receive_cap(state: Rc<ConnectionState<VatId>>, descriptor: cap_descriptor::Reader)
                   -> ::capnp::Result<Option<Box<ClientHook>>>
    {
        match try!(descriptor.which()) {
            cap_descriptor::None(()) => {
                Ok(None)
            }
            cap_descriptor::SenderHosted(sender_hosted) => {
                Ok(Some(ConnectionState::import(state, sender_hosted, false)))
            }
            cap_descriptor::SenderPromise(sender_promise) => {
                Ok(Some(ConnectionState::import(state, sender_promise, true)))
            }
            cap_descriptor::ReceiverHosted(_receiver_hosted) => {
                unimplemented!()
            }
            cap_descriptor::ReceiverAnswer(_receiver_answer) => {
                unimplemented!()
            }
            cap_descriptor::ThirdPartyHosted(_third_party_hosted) => {
                unimplemented!()
            }
        }
    }

    fn receive_caps(state: Rc<ConnectionState<VatId>>,
                    cap_table: ::capnp::struct_list::Reader<cap_descriptor::Owned>)
        -> ::capnp::Result<Vec<Option<Box<ClientHook>>>>
    {
        let mut result = Vec::new();
        for idx in 0..cap_table.len() {
            result.push(try!(ConnectionState::receive_cap(state.clone(), cap_table.get(idx))));
        }
        Ok(result)
    }
}

struct ResponseState<VatId> where VatId: 'static {
    _connection_state: Rc<ConnectionState<VatId>>,
    message: Box<::IncomingMessage>,
    cap_table: ::capnp::capability::ReaderCapTable,
    _question_ref: Rc<RefCell<QuestionRef<VatId>>>,
}

struct Response<VatId> where VatId: 'static {
    state: Rc<ResponseState<VatId>>,
}

impl <VatId> Response<VatId> {
    fn new(connection_state: Rc<ConnectionState<VatId>>,
           question_ref: Rc<RefCell<QuestionRef<VatId>>>,
           message: Box<::IncomingMessage>,
           cap_table_array: Vec<Option<Box<ClientHook>>>) -> Response<VatId> {
        Response {
            state: Rc::new(ResponseState {
                _connection_state: connection_state,
                message: message,
                cap_table: ::capnp::capability::ReaderCapTable::new(cap_table_array),
                _question_ref: question_ref,
            }),
        }
    }
}

impl <VatId> Clone for Response<VatId> {
    fn clone(&self) -> Response<VatId> {
        Response { state: self.state.clone() }
    }
}

impl <VatId> ResponseHook for Response<VatId> {
    fn get<'a>(&'a self) -> ::capnp::Result<any_pointer::Reader<'a>> {
        match try!(try!(try!(self.state.message.get_body()).get_as::<message::Reader>()).which()) {
            message::Return(Ok(ret)) => {
                match try!(ret.which()) {
                    return_::Results(Ok(mut payload)) => {
                        use ::capnp::traits::Imbue;
                        payload.imbue(&self.state.cap_table.hooks);
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
    connection_state: Rc<ConnectionState<VatId>>,
    target: Client<VatId>,
    message: Box<::OutgoingMessage>,
}

fn get_call<'a>(message: &'a mut Box<::OutgoingMessage>)
                -> ::capnp::Result<::rpc_capnp::call::Builder<'a>>
{
    let message_root: message::Builder = try!(try!(message.get_body()).get_as());
    match try!(message_root.which()) {
        message::Call(call) => {
            call
        }
        _ => {
            unimplemented!()
        }
    }
}

impl <VatId> Request<VatId> where VatId: 'static {
    fn new(connection_state: Rc<ConnectionState<VatId>>,
           _size_hint: Option<::capnp::MessageSize>,
           target: Client<VatId>) -> Request<VatId> {

        let message = connection_state.connection.borrow_mut().as_mut().expect("not connected?")
            .new_outgoing_message(100);
        Request {
            connection_state: connection_state,
            target: target,
            message: message,
        }
    }

    fn init_call<'a>(&'a mut self) -> ::rpc_capnp::call::Builder<'a> {
        let message_root: message::Builder = self.message.get_body().unwrap().get_as().unwrap();
        message_root.init_call()
    }

    fn send_internal(connection_state: Rc<ConnectionState<VatId>>,
                     mut message: Box<::OutgoingMessage>,
                     is_tail_call: bool)
                     -> (Rc<RefCell<QuestionRef<VatId>>>, ::gj::Promise<Response<VatId>, Error>)
    {
        // Build the cap table.
        //auto exports = connectionState->writeDescriptors(
        //    capTable.getTable(), callBuilder.getParams());
        let exports = Vec::new();

        // Init the question table.  Do this after writing descriptors to avoid interference.
        let mut question = Question::<VatId>::new();
        question.is_awaiting_return = true;
        question.param_exports = exports;
        question.is_tail_call = is_tail_call;

        let question_id = connection_state.questions.borrow_mut().push(question);

        {
            let mut call_builder: ::rpc_capnp::call::Builder = get_call(&mut message).unwrap();
            // Finish and send.
            call_builder.borrow().set_question_id(question_id);
            if is_tail_call {
                call_builder.get_send_results_to().set_yourself(());
            }
        }
        message.send();

        // Make the result promise.
        let (promise, fulfiller) = ::gj::new_promise_and_fulfiller();
        let question_ref = Rc::new(RefCell::new(
            QuestionRef::new(connection_state.clone(), question_id, fulfiller)));

        match &mut connection_state.questions.borrow_mut().slots[question_id as usize] {
            &mut Some(ref mut q) => {
                q.self_ref = Some(question_ref.clone());
            }
            &mut None => unreachable!(),
        }

        // TODO attach?
        //result.promise = paf.promise.attach(kj::addRef(*result.questionRef));

        (question_ref, promise)
    }
}

impl <VatId> RequestHook for Request<VatId> {
    fn get<'a>(&'a mut self) -> any_pointer::Builder<'a> {
        get_call(&mut self.message).unwrap().get_params().unwrap().get_content()
    }
    fn send<'a>(self: Box<Self>) -> ::capnp::capability::RemotePromise<any_pointer::Owned> {
        let tmp = *self;
        let Request { connection_state, target, mut message } = tmp;
        let write_target_result = {
            let call_builder: ::rpc_capnp::call::Builder = message.get_body().unwrap().get_as().unwrap();
            target.write_target(call_builder.get_target().unwrap())
        };

        match write_target_result {
            Some(_redirect) => {
                // Whoops, this capability has been redirected while we were building the request!
                // We'll have to make a new request and do a copy.  Ick.
                unimplemented!()
            }
            None => {
                let (question_ref, promise) =
                    Request::send_internal(connection_state.clone(), message, false);
                let mut forked_promise = promise.fork();

                // The pipeline must get notified of resolution before the app does to maintain ordering.
                let pipeline = Pipeline::new(connection_state, question_ref,
                                             Some(forked_promise.add_branch()));

                let app_promise = forked_promise.add_branch().map(|response| {
                    Ok(::capnp::capability::Response::new(Box::new(response)))
                });
                ::capnp::capability::RemotePromise {
                    promise: app_promise,
                    pipeline: any_pointer::Pipeline::new(Box::new(pipeline))
                }
            }
        }
    }
}

enum PipelineVariant<VatId> where VatId: 'static {
    Waiting(Rc<RefCell<QuestionRef<VatId>>>),
    _Resolved(Response<VatId>),
    _Broken(::capnp::Error),
}

struct PipelineState<VatId> where VatId: 'static {
    variant: PipelineVariant<VatId>,
    redirect_later: Option<RefCell<::gj::ForkedPromise<Response<VatId>, ::capnp::Error>>>,
    connection_state: Rc<ConnectionState<VatId>>,
}

struct Pipeline<VatId> where VatId: 'static {
    state: Rc<RefCell<PipelineState<VatId>>>,
}

impl <VatId> Pipeline<VatId> {
    fn new(connection_state: Rc<ConnectionState<VatId>>,
           question_ref: Rc<RefCell<QuestionRef<VatId>>>,
           redirect_later: Option<::gj::Promise<Response<VatId>, ::capnp::Error>>)
           -> Pipeline<VatId>
    {
        let state = Rc::new(RefCell::new(PipelineState {
            variant: PipelineVariant::Waiting(question_ref),
            connection_state: connection_state,
            redirect_later: None,
        }));
        match redirect_later {
            Some(redirect_later_promise) => {
                let fork = redirect_later_promise.fork();

/*
                let this = state.clone();
                fork.add_branch().map_else(move |response| {
                    match
                    this.borrow_mut().resolve(response);
                    Ok(())
                });*/

                state.borrow_mut().redirect_later = Some(RefCell::new(fork));
            }
            None => {}
        }
        Pipeline { state: state }
    }

    fn _resolve(&mut self, response: Response<VatId>) {
        match self.state.borrow().variant { PipelineVariant::Waiting( _ ) => (),
                                            _ => panic!("Already resolved?") }
        self.state.borrow_mut().variant = PipelineVariant::_Resolved(response);
    }

}

impl <VatId> PipelineHook for Pipeline<VatId> {
    fn add_ref(&self) -> Box<PipelineHook> {
        Box::new(Pipeline { state: self.state.clone() })
    }
    fn get_pipelined_cap(&self, ops: &[PipelineOp]) -> Box<ClientHook> {
        let mut copy = Vec::new();
        for &op in ops {
            copy.push(op)
        }
        self.get_pipelined_cap_move(copy)
    }
    fn get_pipelined_cap_move(&self, ops: Vec<PipelineOp>) -> Box<ClientHook> {
        match &*self.state.borrow() {
            &PipelineState {variant: PipelineVariant::Waiting(ref question_ref),
                            ref connection_state, ref redirect_later} => {
                // Wrap a PipelineClient in a PromiseClient.
                let pipeline_client =
                    PipelineClient::new(&connection_state, question_ref.clone(), ops.clone());

                match redirect_later {
                    &Some(ref r) => {
                        let resolution_promise = r.borrow_mut().add_branch().map(move |response| {
                           try!(response.get()).get_pipelined_cap(&ops)
                        });
                        let client: Client<VatId> = pipeline_client.into();
                        let promise_client = PromiseClient::new(&connection_state,
                                                                Box::new(client),
                                                                resolution_promise, None);
                        let result: Client<VatId> = promise_client.into();
                        Box::new(result)
                    }
                    &None => {
                        // Oh, this pipeline will never get redirected, so just return the PipelineClient.
                        unimplemented!()
                    }
                }
            }
            &PipelineState {variant: PipelineVariant::_Resolved(ref response), ..} => {
                response.get().unwrap().get_pipelined_cap(&ops[..]).unwrap()
            }
            &PipelineState {variant: PipelineVariant::_Broken(_), ..}  => { unimplemented!() }
        }
    }
}

enum ClientVariant<VatId> where VatId: 'static {
    Import(Rc<RefCell<ImportClient<VatId>>>),
    Pipeline(Rc<RefCell<PipelineClient<VatId>>>),
    Promise(Rc<RefCell<PromiseClient<VatId>>>),
    __Broken(()),
    __NoIntercept(()),
}

struct Client<VatId> where VatId: 'static {
    connection_state: Weak<ConnectionState<VatId>>,
    variant: ClientVariant<VatId>,
}

struct ImportClient<VatId> where VatId: 'static {
    connection_state: Weak<ConnectionState<VatId>>,
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
    fn new(connection_state: &Rc<ConnectionState<VatId>>, import_id: ImportId)
           -> Rc<RefCell<ImportClient<VatId>>> {
        Rc::new(RefCell::new(ImportClient {
            connection_state: Rc::downgrade(connection_state),
            import_id: import_id,
            remote_ref_count: 0,
        }))
    }

    fn add_remote_ref(&mut self) {
        self.remote_ref_count += 1;
    }
}

impl <VatId> From<Rc<RefCell<ImportClient<VatId>>>> for Client<VatId> {
    fn from(client: Rc<RefCell<ImportClient<VatId>>>) -> Client<VatId> {
        let connection_state = client.borrow().connection_state.clone();
        Client { connection_state: connection_state,
                 variant: ClientVariant::Import(client) }
    }
}

/// A ClientHook representing a pipelined promise.  Always wrapped in PromiseClient.
struct PipelineClient<VatId> where VatId: 'static {
    connection_state: Weak<ConnectionState<VatId>>,
    question_ref: Rc<RefCell<QuestionRef<VatId>>>,
    _ops: Vec<PipelineOp>,
}

impl <VatId> PipelineClient<VatId> where VatId: 'static {
    fn new(connection_state: &Rc<ConnectionState<VatId>>,
           question_ref: Rc<RefCell<QuestionRef<VatId>>>,
           ops: Vec<PipelineOp>) -> Rc<RefCell<PipelineClient<VatId>>> {
        Rc::new(RefCell::new(PipelineClient {
            connection_state: Rc::downgrade(connection_state),
            question_ref: question_ref,
            _ops: ops,
        }))
    }
}

impl <VatId> From<Rc<RefCell<PipelineClient<VatId>>>> for Client<VatId> {
    fn from(client: Rc<RefCell<PipelineClient<VatId>>>) -> Client<VatId> {
        let connection_state = client.borrow().connection_state.clone();
        Client { connection_state: connection_state,
                 variant: ClientVariant::Pipeline(client) }
    }
}

/// A ClientHook that initially wraps one client and then, later on, redirects
/// to some other client.
struct PromiseClient<VatId> where VatId: 'static {
    connection_state: Weak<ConnectionState<VatId>>,
    is_resolved: bool,
    cap: Box<ClientHook>,
    import_id: Option<ImportId>,
    fork: ::gj::ForkedPromise<Box<ClientHook>, ::capnp::Error>,
    resolve_self_promise: ::gj::Promise<(), ()>,
    received_call: bool,
}

impl <VatId> PromiseClient<VatId> {
    fn new(connection_state: &Rc<ConnectionState<VatId>>,
           initial: Box<ClientHook>,
           eventual: ::gj::Promise<Box<ClientHook>, ::capnp::Error>,
           import_id: Option<ImportId>) -> Rc<RefCell<PromiseClient<VatId>>> {
        let client = Rc::new(RefCell::new(PromiseClient {
            connection_state: Rc::downgrade(connection_state),
            is_resolved: false,
            cap: initial,
            import_id: import_id,
            fork: eventual.fork(),
            resolve_self_promise: ::gj::Promise::ok(()),
            received_call: false,
        }));
        let resolved = client.borrow_mut().fork.add_branch();
        let weak_this = Rc::downgrade(&client);
        let resolved1 = resolved.map_else(move |result| {
            let this = weak_this.upgrade().expect("impossible");
            match result {
                Ok(v) => {
                    this.borrow_mut().resolve(v, false);
                    Ok(())
                }
                Err(_) => {
                    this.borrow_mut().resolve(unimplemented!(), true);
                    Err(())
                }
            }
        }).eagerly_evaluate();

        client.borrow_mut().resolve_self_promise = resolved1;
        client
    }

    fn resolve(&mut self, replacement: Box<ClientHook>, is_error: bool) {
        let _replacement_brand = replacement.get_brand();
        if false && !is_error {
            // The new capability is hosted locally, not on the remote machine.  And, we had made calls
            // to the promise.  We need to make sure those calls echo back to us before we allow new
            // calls to go directly to the local capability, so we need to set a local embargo and send
            // a `Disembargo` to echo through the peer.
        }
        self.cap = replacement;
        self.is_resolved = true;
    }
}

impl <VatId> Drop for PromiseClient<VatId> {
    fn drop(&mut self) {
        match self.import_id {
            Some(_id) => {
                // This object is representing an import promise.  That means the import table may still
                // contain a pointer back to it.  Remove that pointer.  Note that we have to verify that
                // the import still exists and the pointer still points back to this object because this
                // object may actually outlive the import.

                // TODO
            }
            None => {}
        }
    }
}

impl <VatId> From<Rc<RefCell<PromiseClient<VatId>>>> for Client<VatId> {
    fn from(client: Rc<RefCell<PromiseClient<VatId>>>) -> Client<VatId> {
        let connection_state = client.borrow().connection_state.clone();
        Client { connection_state: connection_state,
                 variant: ClientVariant::Promise(client) }
    }
}

impl <VatId> Client<VatId> {
    fn write_target(&self, mut target: ::rpc_capnp::message_target::Builder)
                    -> Option<Box<ClientHook>>
    {
        match &self.variant {
            &ClientVariant::Import(ref import_client) => {
                target.set_imported_cap(import_client.borrow().import_id);
                None
            }
            &ClientVariant::Pipeline(ref pipeline_client) => {
                let mut builder = target.init_promised_answer();
                let question_ref = &pipeline_client.borrow().question_ref;
                builder.set_question_id(question_ref.borrow().id);
                // adopt_transform
                None
            }
            &ClientVariant::Promise(ref promise_client) => {
                promise_client.borrow_mut().received_call = true;
                self.connection_state.upgrade().expect("no connection?").write_target(
                    &*promise_client.borrow().cap, target).unwrap()
            }
            _ => {
                unimplemented!()
            }
        }
    }
}

impl <VatId> Clone for Client<VatId> {
    fn clone(&self) -> Client<VatId> {
        let variant = match &self.variant {
            &ClientVariant::Import(ref import_client) => {
                ClientVariant::Import(import_client.clone())
            }
            &ClientVariant::Pipeline(ref pipeline_client) => {
                ClientVariant::Pipeline(pipeline_client.clone())
            }
            &ClientVariant::Promise(ref promise_client) => {
                ClientVariant::Promise(promise_client.clone())
            }
            _ => {
                unimplemented!()
            }
        };
        Client { connection_state: self.connection_state.clone(), variant: variant}
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
        let mut request = Request::new(self.connection_state.upgrade().expect("no connection?"),
                                       size_hint, self.clone());
        {
            let mut call_builder = request.init_call();
            call_builder.set_interface_id(interface_id);
            call_builder.set_method_id(method_id);
        }

        ::capnp::capability::Request::new(Box::new(request))
    }

    fn call(&self, interface_id: u64, method_id: u16, params: Box<ParamsHook>, _results: Box<ResultsHook>) {
        let mut request = self.new_call(interface_id, method_id,
                                        Some(params.get().total_size().unwrap()));
        request.init().set_as(params.get()).unwrap();

        // We can and should propagate cancellation.
        // context -> allowCancellation();

        unimplemented!()
    }

    fn get_brand(&self) -> usize {
        self.connection_state.upgrade().expect("no connection?").get_brand()
    }

    fn write_target(&self, target: any_pointer::Builder) -> Option<Box<ClientHook>>
    {
        self.write_target(target.init_as())
    }

    fn get_descriptor(&self) -> Box<::std::any::Any> {
        unimplemented!()
    }
}
