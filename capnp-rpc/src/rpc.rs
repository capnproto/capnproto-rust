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

use std::pin::Pin;
use std::task::{Context, Poll};

use capnp::any_pointer;
use capnp::capability::Promise;
use capnp::private::capability::{
    ClientHook, ParamsHook, PipelineHook, PipelineOp, RequestHook, ResponseHook, ResultsHook,
};
use capnp::Error;

use futures::channel::oneshot;
use futures::{future, Future, FutureExt, TryFutureExt};

use std::cell::{Cell, RefCell};
use std::cmp::Reverse;
use std::collections::binary_heap::BinaryHeap;
use std::collections::hash_map::{self, HashMap};
use std::mem;
use std::rc::{Rc, Weak};

use crate::attach::Attach;
use crate::local::ResultsDoneHook;
use crate::rpc_capnp::{
    bootstrap, call, cap_descriptor, disembargo, exception, finish, message, message_target,
    payload, promised_answer, resolve, return_,
};
use crate::task_set::TaskSet;
use crate::{broken, local, queued};

pub(crate) type QuestionId = u32;
pub(crate) type AnswerId = QuestionId;
pub(crate) type ExportId = u32;
pub(crate) type ImportId = ExportId;

pub(crate) struct ImportTable<T> {
    slots: HashMap<u32, T>,
}

impl<T> ImportTable<T> {
    pub(crate) fn new() -> Self {
        Self {
            slots: HashMap::new(),
        }
    }
}

struct ExportTable<T> {
    slots: Vec<Option<T>>,

    // prioritize lower values
    free_ids: BinaryHeap<Reverse<u32>>,
}

struct ExportTableIter<'a, T>
where
    T: 'a,
{
    table: &'a ExportTable<T>,
    idx: usize,
}

impl<'a, T> ::std::iter::Iterator for ExportTableIter<'a, T>
where
    T: 'a,
{
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        while self.idx < self.table.slots.len() {
            let idx = self.idx;
            self.idx += 1;
            if let Some(v) = &self.table.slots[idx] {
                return Some(v);
            }
        }
        None
    }
}

impl<T> ExportTable<T> {
    pub(crate) fn new() -> Self {
        Self {
            slots: Vec::new(),
            free_ids: BinaryHeap::new(),
        }
    }

    pub(crate) fn erase(&mut self, id: u32) {
        self.slots[id as usize] = None;
        self.free_ids.push(Reverse(id));
    }

    pub(crate) fn push(&mut self, val: T) -> u32 {
        match self.free_ids.pop() {
            Some(Reverse(id)) => {
                self.slots[id as usize] = Some(val);
                id
            }
            None => {
                self.slots.push(Some(val));
                self.slots.len() as u32 - 1
            }
        }
    }

    pub(crate) fn find(&mut self, id: u32) -> Option<&mut T> {
        let idx = id as usize;
        if idx < self.slots.len() {
            self.slots[idx].as_mut()
        } else {
            None
        }
    }

    pub(crate) fn iter(&self) -> ExportTableIter<'_, T> {
        ExportTableIter {
            table: self,
            idx: 0,
        }
    }
}

struct Question<VatId>
where
    VatId: 'static,
{
    is_awaiting_return: bool,

    #[allow(dead_code)]
    param_exports: Vec<ExportId>,

    #[allow(dead_code)]
    is_tail_call: bool,

    /// The local QuestionRef, set to None when it is destroyed.
    self_ref: Option<Weak<RefCell<QuestionRef<VatId>>>>,

    /// If true, don't send a Finish message.
    skip_finish: bool,
}

impl<VatId> Question<VatId> {
    fn new() -> Self {
        Self {
            is_awaiting_return: true,
            param_exports: Vec::new(),
            is_tail_call: false,
            self_ref: None,
            skip_finish: false,
        }
    }
}

/// A reference to an entry on the question table.  Used to detect when the `Finish` message
/// can be sent.
struct QuestionRef<VatId>
where
    VatId: 'static,
{
    connection_state: Rc<ConnectionState<VatId>>,
    id: QuestionId,
    fulfiller: Option<oneshot::Sender<Promise<Response<VatId>, Error>>>,
    pipelines: Vec<Weak<RefCell<PipelineState<VatId>>>>,
}

impl<VatId> QuestionRef<VatId> {
    fn new(
        state: Rc<ConnectionState<VatId>>,
        id: QuestionId,
        fulfiller: oneshot::Sender<Promise<Response<VatId>, Error>>,
    ) -> Self {
        Self {
            connection_state: state,
            id,
            fulfiller: Some(fulfiller),
            pipelines: Vec::new(),
        }
    }
    fn fulfill(&mut self, response: Promise<Response<VatId>, Error>) {
        if let Some(fulfiller) = self.fulfiller.take() {
            let _ = fulfiller.send(response);
        }
    }

    fn reject(&mut self, err: Error) {
        if let Some(fulfiller) = self.fulfiller.take() {
            let _ = fulfiller.send(Promise::err(err.clone()));
        }
        // The RPC driver may have unwound. Its scheduled pipeline resolvers
        // cannot be relied on to release the application's response barrier.
        for pipeline in self.pipelines.drain(..).filter_map(|p| p.upgrade()) {
            PipelineState::resolve(&pipeline, Err(err.clone()));
        }
    }
}

impl<VatId> Drop for QuestionRef<VatId> {
    fn drop(&mut self) {
        let mut questions = self.connection_state.questions.borrow_mut();
        let Some(q) = &mut questions.slots[self.id as usize] else {
            unreachable!()
        };
        if let Ok(ref mut c) = *self.connection_state.connection.borrow_mut() {
            if !q.skip_finish {
                let mut message = c.new_outgoing_message(5);
                {
                    let root: message::Builder = message.get_body().unwrap().init_as();
                    let mut builder = root.init_finish();
                    builder.set_question_id(self.id);

                    // If we're still awaiting a return, then this request is being
                    // canceled, and we're going to ignore any capabilities in the return
                    // message, so set releaseResultCaps true. If we already received the
                    // return, then we've already built local proxies for the caps and will
                    // send Release messages when those are destroyed.
                    builder.set_release_result_caps(q.is_awaiting_return);
                }
                let _ = message.send();
            }
        }

        if q.is_awaiting_return {
            // Still waiting for return, so just remove the QuestionRef pointer from the table.
            q.self_ref = None;
        } else {
            // Call has already returned, so we can now remove it from the table.
            questions.erase(self.id)
        }
    }
}

struct Answer<VatId>
where
    VatId: 'static,
{
    return_has_been_sent: bool,

    // Send pipelined calls here.  Becomes null as soon as a `Finish` is received.
    pipeline: Option<Box<dyn PipelineHook>>,

    // For locally-redirected calls (Call.sendResultsTo.yourself), this is a promise for the call
    // result, to be picked up by a subsequent `Return`.
    redirected_results: Option<Promise<Response<VatId>, Error>>,

    received_finish: Rc<Cell<bool>>,
    call_completion_promise: Option<Promise<(), Error>>,

    // List of exports that were sent in the results.  If the finish has `releaseResultCaps` these
    // will need to be released.
    result_exports: Vec<ExportId>,
}

impl<VatId> Answer<VatId> {
    fn new() -> Self {
        Self {
            return_has_been_sent: false,
            pipeline: None,
            redirected_results: None,
            received_finish: Rc::new(Cell::new(false)),
            call_completion_promise: None,
            result_exports: Vec::new(),
        }
    }
}

pub(crate) struct Export {
    refcount: u32,

    /// The capability identity whose canonical export is this entry. Cache it
    /// so table updates never need to call application code to recover the key.
    canonical: Option<usize>,

    client_hook: Box<dyn ClientHook>,

    // If this export is a promise (not a settled capability), the `resolve_op` represents the
    // ongoing operation to wait for that promise to resolve and then send a `Resolve` message.
    resolve_op: Promise<(), Error>,
}

impl Export {
    fn new(client_hook: Box<dyn ClientHook>) -> Self {
        Self {
            refcount: 1,
            canonical: None,
            client_hook,
            resolve_op: Promise::err(Error::failed("no resolve op".to_string())),
        }
    }
}

pub(crate) struct Import<VatId>
where
    VatId: 'static,
{
    import_client: Weak<RefCell<ImportClient<VatId>>>,

    // Either a copy of importClient, or, in the case of promises, the wrapping PromiseClient.
    // Becomes null when it is discarded *or* when the import is destroyed (e.g. the promise is
    // resolved and the import is no longer needed).
    app_client: Option<WeakClient<VatId>>,

    // If non-null, the import is a promise.
    promise_client_to_resolve: Option<Weak<RefCell<PromiseClient<VatId>>>>,
}

impl<VatId> Import<VatId> {
    fn new(import_client: &Rc<RefCell<ImportClient<VatId>>>) -> Self {
        Self {
            import_client: Rc::downgrade(import_client),
            app_client: None,
            promise_client_to_resolve: None,
        }
    }
}

struct Embargo {
    fulfiller: Option<oneshot::Sender<Result<(), Error>>>,
}

impl Embargo {
    fn new(fulfiller: oneshot::Sender<Result<(), Error>>) -> Self {
        Self {
            fulfiller: Some(fulfiller),
        }
    }
}

fn to_pipeline_ops(
    ops: ::capnp::struct_list::Reader<promised_answer::op::Owned>,
) -> ::capnp::Result<Vec<PipelineOp>> {
    let mut result = Vec::new();
    for op in ops {
        match op.which()? {
            promised_answer::op::Noop(()) => {
                result.push(PipelineOp::Noop);
            }
            promised_answer::op::GetPointerField(idx) => {
                result.push(PipelineOp::GetPointerField(idx));
            }
        }
    }
    Ok(result)
}

fn from_error(error: &Error, mut builder: exception::Builder) {
    let typ = match error.kind {
        ::capnp::ErrorKind::Failed => exception::Type::Failed,
        ::capnp::ErrorKind::Overloaded => exception::Type::Overloaded,
        ::capnp::ErrorKind::Disconnected => exception::Type::Disconnected,
        ::capnp::ErrorKind::Unimplemented => exception::Type::Unimplemented,
        ::capnp::ErrorKind::SettingDynamicCapabilitiesIsUnsupported => {
            exception::Type::Unimplemented
        }
        _ => exception::Type::Failed,
    };
    builder.set_type(typ);
    match error.kind {
        ::capnp::ErrorKind::Failed
        | ::capnp::ErrorKind::Overloaded
        | ::capnp::ErrorKind::Disconnected
        | ::capnp::ErrorKind::Unimplemented => {
            builder.set_reason(&error.extra);
        }
        _ => {
            // There is extra information in `error.kind` that is not
            // captured by `typ`. We call `error.to_string()` to allow that
            // information to be recorded in the `reason` field.
            builder.set_reason(error.to_string());
        }
    }
}

fn remote_exception_to_error(exception: exception::Reader) -> Error {
    let (kind, reason) = match (exception.get_type(), exception.get_reason()) {
        (Ok(exception::Type::Failed), Ok(reason)) => (::capnp::ErrorKind::Failed, reason),
        (Ok(exception::Type::Overloaded), Ok(reason)) => (::capnp::ErrorKind::Overloaded, reason),
        (Ok(exception::Type::Disconnected), Ok(reason)) => {
            (::capnp::ErrorKind::Disconnected, reason)
        }
        (Ok(exception::Type::Unimplemented), Ok(reason)) => {
            (::capnp::ErrorKind::Unimplemented, reason)
        }
        _ => (::capnp::ErrorKind::Failed, "(malformed error)".into()),
    };
    let reason_str = reason
        .to_str()
        .unwrap_or("<malformed utf-8 in error reason>");
    Error {
        extra: format!("remote exception: {reason_str}"),
        kind,
    }
}

pub(crate) struct ConnectionErrorHandler<VatId>
where
    VatId: 'static,
{
    weak_state: Weak<ConnectionState<VatId>>,
}

impl<VatId> ConnectionErrorHandler<VatId> {
    fn new(weak_state: Weak<ConnectionState<VatId>>) -> Self {
        Self { weak_state }
    }
}

impl<VatId> crate::task_set::TaskReaper<capnp::Error> for ConnectionErrorHandler<VatId> {
    fn task_failed(&mut self, error: ::capnp::Error) {
        if let Some(state) = self.weak_state.upgrade() {
            state.disconnect(error)
        }
    }
}

// Catch each teardown target independently and preserve the first panic's
// original payload. A secondary payload may itself have a panicking Drop.
fn catch_disconnect_panic<T>(
    first: &mut Option<Box<dyn std::any::Any + Send>>,
    action: impl FnOnce() -> T,
) -> Option<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)) {
        Ok(value) => Some(value),
        Err(panic) => {
            if first.is_none() {
                *first = Some(panic);
            } else if let Err(nested) =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(panic)))
            {
                // There is no safe general way to destroy a recursively
                // panicking payload. Retain the causal failure, not this one.
                mem::forget(nested);
            }
            None
        }
    }
}

pub struct ConnectionState<VatId>
where
    VatId: 'static,
{
    bootstrap_cap: Box<dyn ClientHook>,
    exports: RefCell<ExportTable<Export>>,
    questions: RefCell<ExportTable<Question<VatId>>>,
    answers: RefCell<ImportTable<Answer<VatId>>>,
    imports: RefCell<ImportTable<Import<VatId>>>,

    /// Exports keyed by ClientHook::get_ptr().
    exports_by_cap: RefCell<HashMap<usize, ExportId>>,

    embargoes: RefCell<ExportTable<Embargo>>,

    tasks: RefCell<Option<crate::task_set::TaskSetHandle<capnp::Error>>>,
    connection: RefCell<::std::result::Result<Box<dyn crate::Connection<VatId>>, ::capnp::Error>>,
    disconnect_fulfiller: RefCell<Option<oneshot::Sender<Promise<(), Error>>>>,

    // Set when `disconnect()` is called. Resolves once the connection's `shutdown()`
    // has completed. `Disconnector` futures wait on a clone of this.
    disconnect_promise: RefCell<Option<future::Shared<Promise<(), Error>>>>,

    client_downcast_map: RefCell<HashMap<usize, WeakClient<VatId>>>,
}

impl<VatId> ConnectionState<VatId> {
    pub fn new(
        bootstrap_cap: Box<dyn ClientHook>,
        connection: Box<dyn crate::Connection<VatId>>,
        disconnect_fulfiller: oneshot::Sender<Promise<(), Error>>,
    ) -> (TaskSet<Error>, Rc<Self>) {
        let state = Rc::new(Self {
            bootstrap_cap,
            exports: RefCell::new(ExportTable::new()),
            questions: RefCell::new(ExportTable::new()),
            answers: RefCell::new(ImportTable::new()),
            imports: RefCell::new(ImportTable::new()),
            exports_by_cap: RefCell::new(HashMap::new()),
            embargoes: RefCell::new(ExportTable::new()),
            tasks: RefCell::new(None),
            connection: RefCell::new(Ok(connection)),
            disconnect_fulfiller: RefCell::new(Some(disconnect_fulfiller)),
            disconnect_promise: RefCell::new(None),
            client_downcast_map: RefCell::new(HashMap::new()),
        });
        let (mut handle, tasks) =
            TaskSet::new(Box::new(ConnectionErrorHandler::new(Rc::downgrade(&state))));

        handle.add(Self::message_loop(Rc::downgrade(&state)));
        *state.tasks.borrow_mut() = Some(handle);
        (tasks, state)
    }

    fn new_outgoing_message(
        &self,
        first_segment_words: u32,
    ) -> capnp::Result<Box<dyn crate::OutgoingMessage>> {
        match self.connection.borrow_mut().as_mut() {
            Err(e) => Err(e.clone()),
            Ok(c) => Ok(c.new_outgoing_message(first_segment_words)),
        }
    }

    fn check_connected(&self) -> capnp::Result<()> {
        self.connection
            .borrow()
            .as_ref()
            .map(|_| ())
            .map_err(Clone::clone)
    }

    fn disconnect(&self, error: ::capnp::Error) {
        let mut connection = {
            let mut slot = self.connection.borrow_mut();
            if slot.is_err() {
                return;
            }
            // Publish the terminal cause before any user-controlled drop or
            // callback can reenter this connection.
            match mem::replace(&mut *slot, Err(error.clone())) {
                Ok(connection) => connection,
                Err(_) => unreachable!(),
            }
        };
        let fulfiller = self.disconnect_fulfiller.borrow_mut().take();

        // Extract every callback/drop target before invoking any of them.
        // Resolving a pipeline can release its QuestionRef and revisit tables.
        let questions: Vec<_> = self
            .questions
            .borrow()
            .iter()
            .filter_map(|q| q.self_ref.as_ref().and_then(Weak::upgrade))
            .collect();
        let pipelines: Vec<_> = self
            .answers
            .borrow_mut()
            .slots
            .values_mut()
            .filter_map(|answer| answer.pipeline.take())
            .collect();
        let exports = mem::replace(&mut *self.exports.borrow_mut(), ExportTable::new());
        self.exports_by_cap.borrow_mut().clear();
        let mut clients = Vec::new();
        let mut resolve_ops = Vec::new();
        for export in exports.slots.into_iter().flatten() {
            clients.push(export.client_hook);
            resolve_ops.push(export.resolve_op);
        }
        let imports: Vec<_> = self
            .imports
            .borrow_mut()
            .slots
            .values_mut()
            .filter_map(|import| import.promise_client_to_resolve.take())
            .filter_map(|client| client.upgrade())
            .collect();
        let embargoes = mem::replace(&mut *self.embargoes.borrow_mut(), ExportTable::new());

        let mut first_panic = None;
        for question in questions {
            catch_disconnect_panic(&mut first_panic, || {
                question.borrow_mut().reject(error.clone())
            });
        }
        for import in imports {
            catch_disconnect_panic(&mut first_panic, || {
                import.borrow_mut().resolve(Err(error.clone()))
            });
        }
        for embargo in embargoes.slots.into_iter().flatten() {
            if let Some(fulfiller) = embargo.fulfiller {
                catch_disconnect_panic(&mut first_panic, || {
                    let _ = fulfiller.send(Err(error.clone()));
                });
            }
        }

        // The removed transport is usable for the final abort/shutdown while
        // all application-facing paths already observe a terminal connection.
        catch_disconnect_panic(&mut first_panic, || {
            let mut message = connection.new_outgoing_message(100);
            from_error(
                &error,
                message
                    .get_body()
                    .unwrap()
                    .init_as::<message::Builder>()
                    .init_abort(),
            );
            let _ = message.send();
        });
        let shutdown =
            catch_disconnect_panic(&mut first_panic, || connection.shutdown(Err(error.clone())))
                .unwrap_or_else(|| Promise::err(error.clone()));
        let shutdown = shutdown.then(|r| match r {
            Ok(()) => Promise::ok(()),
            Err(e) if e.kind == ::capnp::ErrorKind::Disconnected => Promise::ok(()),
            Err(e) => Promise::err(e),
        });

        let shutdown = Promise::from_future(shutdown.attach(connection)).shared();
        *self.disconnect_promise.borrow_mut() = Some(shutdown.clone());

        // Each ownership edge gets its own unwind boundary. In particular,
        // dropping a Vec while unwinding must not trigger a second destructor
        // panic and abort before unrelated exports have been released.
        for pipeline in pipelines {
            catch_disconnect_panic(&mut first_panic, || drop(pipeline));
        }
        for client in clients {
            catch_disconnect_panic(&mut first_panic, || drop(client));
        }
        for resolve_op in resolve_ops {
            catch_disconnect_panic(&mut first_panic, || drop(resolve_op));
        }

        // Deliver completion before reporting the first causal panic. A closed
        // receiver can drop the transport, so guard that ownership release too.
        catch_disconnect_panic(&mut first_panic, || {
            if let Some(fulfiller) = fulfiller {
                let _ = fulfiller.send(Promise::from_future(shutdown));
            }
        });
        if let Some(panic) = first_panic {
            std::panic::resume_unwind(panic);
        }
    }

    // Transform a future into a promise that gets executed even if it is never polled.
    // Dropping the returned promise cancels the computation.
    fn eagerly_evaluate<T, F>(&self, task: F) -> Promise<T, Error>
    where
        F: Future<Output = Result<T, Error>> + 'static + Unpin,
        T: 'static,
    {
        let (tx, rx) = oneshot::channel::<Result<T, Error>>();
        let (tx2, rx2) = oneshot::channel::<()>();
        let f1 = Box::pin(task.map(move |r| {
            let _ = tx.send(r);
        })) as Pin<Box<dyn Future<Output = ()> + Unpin>>;
        let f2 = Box::pin(rx2.map(drop)) as Pin<Box<dyn Future<Output = ()> + Unpin>>;

        self.add_task(future::select(f1, f2).map(|_| Ok(())));
        Promise::from_future(rx.map_err(crate::canceled_to_error).map(|r| {
            drop(tx2);
            r?
        }))
    }

    fn add_task<F>(&self, task: F)
    where
        F: Future<Output = Result<(), Error>> + 'static,
    {
        if let Some(ref mut tasks) = *self.tasks.borrow_mut() {
            tasks.add(task);
        }
    }

    pub fn bootstrap(state: &Rc<Self>) -> Box<dyn ClientHook> {
        if let Err(error) = state.check_connected() {
            return broken::new_cap(error);
        }
        let question_id = state.questions.borrow_mut().push(Question::new());

        let (fulfiller, promise) = oneshot::channel();
        let promise = promise.map_err(crate::canceled_to_error);
        let promise = promise.and_then(|response_promise| response_promise);
        let question_ref = Rc::new(RefCell::new(QuestionRef::new(
            state.clone(),
            question_id,
            fulfiller,
        )));
        let promise = promise.attach(question_ref.clone());
        match state.questions.borrow_mut().slots[question_id as usize] {
            Some(ref mut q) => {
                q.self_ref = Some(Rc::downgrade(&question_ref));
            }
            None => unreachable!(),
        }
        match *state.connection.borrow_mut() {
            Ok(ref mut c) => {
                let mut message = c.new_outgoing_message(5);
                {
                    let mut builder = message
                        .get_body()
                        .unwrap()
                        .init_as::<message::Builder>()
                        .init_bootstrap();
                    builder.set_question_id(question_id);
                }
                let _ = message.send();
            }
            Err(_) => panic!(),
        }

        let pipeline = Pipeline::new(state, question_ref, Some(Promise::from_future(promise)));
        pipeline.get_pipelined_cap_move(Vec::new())
    }

    fn message_loop(weak_state: Weak<Self>) -> Promise<(), capnp::Error> {
        let Some(state) = weak_state.upgrade() else {
            return Promise::err(Error::disconnected(
                "message loop cannot continue without a connection".into(),
            ));
        };

        let promise = match *state.connection.borrow_mut() {
            Err(_) => return Promise::ok(()),
            Ok(ref mut connection) => connection.receive_incoming_message(),
        };

        Promise::from_future(async move {
            match promise.await? {
                Some(m) => {
                    Self::handle_message(&weak_state, m)?;
                    weak_state
                        .upgrade()
                        .expect("message loop outlived connection state?")
                        .add_task(Self::message_loop(weak_state));
                }
                None => {
                    weak_state
                        .upgrade()
                        .expect("message loop outlived connection state?")
                        .disconnect(Error::disconnected("Peer disconnected.".to_string()));
                }
            }
            Ok(())
        })
    }

    fn send_unimplemented(
        connection_state: &Rc<Self>,
        message: &dyn crate::IncomingMessage,
    ) -> capnp::Result<()> {
        let mut out_message = connection_state.new_outgoing_message(50)?; // XXX size hint
        {
            let mut root: message::Builder = out_message.get_body()?.get_as()?;
            root.set_unimplemented(message.get_body()?.get_as()?)?;
        }
        let _ = out_message.send();
        Ok(())
    }

    fn handle_unimplemented(
        connection_state: &Rc<Self>,
        message: message::Reader,
    ) -> capnp::Result<()> {
        match message.which()? {
            message::Resolve(resolve) => {
                let resolve = resolve?;
                match resolve.which()? {
                    resolve::Cap(c) => match c?.which()? {
                        cap_descriptor::None(()) => (),
                        cap_descriptor::SenderHosted(export_id) => {
                            connection_state.release_export(export_id, 1)?;
                        }
                        cap_descriptor::SenderPromise(export_id) => {
                            connection_state.release_export(export_id, 1)?;
                        }
                        cap_descriptor::ReceiverAnswer(_) | cap_descriptor::ReceiverHosted(_) => (),
                        cap_descriptor::ThirdPartyHosted(_) => {
                            return Err(Error::failed(
                                "Peer claims we resolved a ThirdPartyHosted cap.".to_string(),
                            ));
                        }
                    },
                    resolve::Exception(_) => (),
                }
            }
            _ => {
                return Err(Error::failed(
                    "Peer did not implement required RPC message type.".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn handle_bootstrap(
        connection_state: &Rc<Self>,
        bootstrap: bootstrap::Reader,
    ) -> capnp::Result<()> {
        use ::capnp::traits::ImbueMut;

        let answer_id = bootstrap.get_question_id();
        if connection_state.connection.borrow().is_err() {
            // Disconnected; ignore.
            return Ok(());
        }

        let mut response = connection_state.new_outgoing_message(10)?;

        let result_exports = {
            let mut ret = response
                .get_body()?
                .init_as::<message::Builder>()
                .init_return();
            ret.set_answer_id(answer_id);

            let cap = connection_state.bootstrap_cap.clone();
            let mut cap_table = Vec::new();
            let mut payload = ret.init_results();
            {
                let mut content = payload.reborrow().get_content();
                content.imbue_mut(&mut cap_table);
                content.set_as_capability(cap);
            }
            assert_eq!(cap_table.len(), 1);

            Self::write_descriptors(connection_state, &cap_table, payload)?
        };

        let slots = &mut connection_state.answers.borrow_mut().slots;
        let hash_map::Entry::Vacant(slot) = slots.entry(answer_id) else {
            connection_state.release_exports(&result_exports)?;
            return Err(Error::failed("questionId is already in use".to_string()));
        };
        let mut answer = Answer::new();
        answer.return_has_been_sent = true;
        answer.result_exports = result_exports;
        answer.pipeline = Some(Box::new(SingleCapPipeline::new(
            connection_state.bootstrap_cap.clone(),
        )));
        slot.insert(answer);

        let _ = response.send();
        Ok(())
    }

    fn handle_finish(connection_state: &Rc<Self>, finish: finish::Reader) -> capnp::Result<()> {
        let mut exports_to_release = Vec::new();
        let answer_id = finish.get_question_id();

        let answers_slots = &mut connection_state.answers.borrow_mut().slots;
        match answers_slots.entry(answer_id) {
            hash_map::Entry::Vacant(_) => {
                // The `Finish` message targets a question ID that isn't present in our answer table.
                // Probably, we sent a `Return` with `noFinishNeeded = true`, but the other side didn't
                // recognize this hint and sent a `Finish` anyway, or the `Finish` was already in-flight at
                // the time we sent the `Return`. We can silently ignore this.
            }
            hash_map::Entry::Occupied(mut entry) => {
                let answer = entry.get_mut();
                answer.received_finish.set(true);

                if finish.get_release_result_caps() {
                    exports_to_release = ::std::mem::take(&mut answer.result_exports);
                }

                // If the pipeline has not been cloned, the following two lines cancel the call.
                answer.pipeline.take();
                answer.call_completion_promise.take();

                if answer.return_has_been_sent {
                    entry.remove();
                }
            }
        }

        connection_state.release_exports(&exports_to_release)?;
        Ok(())
    }

    fn handle_resolve(connection_state: &Rc<Self>, resolve: resolve::Reader) -> capnp::Result<()> {
        let replacement_or_error = match resolve.which()? {
            resolve::Cap(c) => match Self::receive_cap(connection_state, c?)? {
                Some(cap) => Ok(cap),
                None => {
                    return Err(Error::failed(
                        "'Resolve' contained 'CapDescriptor.none'.".to_string(),
                    ));
                }
            },
            resolve::Exception(e) => {
                // We can't set `replacement` to a new broken cap here because this will
                // confuse PromiseClient::Resolve() into thinking that the remote
                // promise resolved to a local capability and therefore a Disembargo is
                // needed. We must actually reject the promise.
                Err(remote_exception_to_error(e?))
            }
        };

        // If the import is in the table, fulfill it.
        let slots = &mut connection_state.imports.borrow_mut().slots;
        if let Some(import) = slots.get_mut(&resolve.get_promise_id()) {
            match import.promise_client_to_resolve.take() {
                Some(weak_promise_client) => {
                    if let Some(promise_client) = weak_promise_client.upgrade() {
                        promise_client.borrow_mut().resolve(replacement_or_error);
                    }
                }
                None => {
                    return Err(Error::failed(
                        "Got 'Resolve' for a non-promise import.".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn handle_disembargo(
        connection_state: &Rc<Self>,
        disembargo: disembargo::Reader,
    ) -> capnp::Result<()> {
        let context = disembargo.get_context();
        match context.which()? {
            disembargo::context::SenderLoopback(embargo_id) => {
                let mut target = connection_state.get_message_target(disembargo.get_target()?)?;
                while let Some(resolved) = target.get_resolved() {
                    target = resolved;
                }

                if target.get_brand() != connection_state.get_brand() {
                    return Err(Error::failed(
                        "'Disembargo' of type 'senderLoopback' sent to an object that does not point \
                         back to the sender.".to_string()));
                }

                let connection_state_ref = connection_state.clone();
                let connection_state_ref1 = connection_state.clone();
                let task = async move {
                    if let Ok(ref mut c) = *connection_state_ref.connection.borrow_mut() {
                        let mut message = c.new_outgoing_message(100); // TODO estimate size
                        {
                            let root: message::Builder = message.get_body()?.init_as();
                            let mut disembargo = root.init_disembargo();
                            disembargo
                                .reborrow()
                                .init_context()
                                .set_receiver_loopback(embargo_id);

                            let redirect =
                                match Client::from_ptr(target.get_ptr(), &connection_state_ref1) {
                                    Some(c) => c.write_target(disembargo.init_target()),
                                    None => unreachable!(),
                                };
                            if redirect.is_some() {
                                return Err(Error::failed(
                                    "'Disembargo' of type 'senderLoopback' sent to an object that \
                                     does not appear to have been the subject of a previous \
                                     'Resolve' message."
                                        .to_string(),
                                ));
                            }
                        }
                        let _ = message.send();
                    }
                    Ok(())
                };
                connection_state.add_task(task);
            }
            disembargo::context::ReceiverLoopback(embargo_id) => {
                if let Some(embargo) = connection_state.embargoes.borrow_mut().find(embargo_id) {
                    let fulfiller = embargo.fulfiller.take().unwrap();
                    let _ = fulfiller.send(Ok(()));
                } else {
                    return Err(Error::failed(
                        "Invalid embargo ID in `Disembargo.context.receiverLoopback".to_string(),
                    ));
                }
                connection_state.embargoes.borrow_mut().erase(embargo_id);
            }
            disembargo::context::Accept(_) | disembargo::context::Provide(_) => {
                return Err(Error::unimplemented(
                    "Disembargo::Context::Provide/Accept not implemented".to_string(),
                ));
            }
        }
        Ok(())
    }

    fn handle_message(
        weak_state: &Weak<Self>,
        message: Box<dyn crate::IncomingMessage>,
    ) -> ::capnp::Result<()> {
        let Some(connection_state) = weak_state.upgrade() else {
            return Err(Error::disconnected(
                "handle_message() cannot continue without a connection".into(),
            ));
        };

        let reader = message.get_body()?.get_as::<message::Reader>()?;
        match reader.which() {
            Ok(message::Unimplemented(message)) => {
                Self::handle_unimplemented(&connection_state, message?)?
            }
            Ok(message::Abort(abort)) => return Err(remote_exception_to_error(abort?)),
            Ok(message::Bootstrap(bootstrap)) => {
                Self::handle_bootstrap(&connection_state, bootstrap?)?
            }
            Ok(message::Call(call)) => {
                let call = call?;
                let capability = connection_state.get_message_target(call.get_target()?)?;
                let (interface_id, method_id, question_id, cap_table_array, redirect_results) = {
                    let redirect_results = match call.get_send_results_to().which()? {
                        call::send_results_to::Caller(()) => false,
                        call::send_results_to::Yourself(()) => true,
                        call::send_results_to::ThirdParty(_) => {
                            return Err(Error::failed(
                                "Unsupported `Call.sendResultsTo`.".to_string(),
                            ))
                        }
                    };
                    let payload = call.get_params()?;

                    (
                        call.get_interface_id(),
                        call.get_method_id(),
                        call.get_question_id(),
                        Self::receive_caps(&connection_state, payload.get_cap_table()?)?,
                        redirect_results,
                    )
                };

                if connection_state
                    .answers
                    .borrow()
                    .slots
                    .contains_key(&question_id)
                {
                    return Err(Error::failed(format!(
                        "Received a new call on in-use question id {question_id}"
                    )));
                }

                let params = Params::new(message, cap_table_array);

                let answer = Answer::new();

                let (results_inner_fulfiller, results_inner_promise) = oneshot::channel();
                let results_inner_promise = results_inner_promise.map_err(crate::canceled_to_error);

                let (pipeline_sender, mut pipeline) = queued::Pipeline::new();
                let results = Results::new(
                    &connection_state,
                    question_id,
                    redirect_results,
                    results_inner_fulfiller,
                    answer.received_finish.clone(),
                    Some(pipeline_sender.weak_clone()),
                );

                let (redirected_results_done_promise, redirected_results_done_fulfiller) =
                    if redirect_results {
                        let (f, p) = oneshot::channel::<Result<Response<VatId>, Error>>();
                        let p = p.map_err(crate::canceled_to_error).and_then(future::ready);
                        (Some(Promise::from_future(p)), Some(f))
                    } else {
                        (None, None)
                    };

                {
                    let slots = &mut connection_state.answers.borrow_mut().slots;
                    let hash_map::Entry::Vacant(slot) = slots.entry(question_id) else {
                        return Err(Error::failed("questionId is already in use".to_string()));
                    };
                    slot.insert(answer);
                }

                let call_promise =
                    capability.call(interface_id, method_id, Box::new(params), Box::new(results));

                let promise = call_promise
                    .then(move |call_result| {
                        results_inner_promise.then(move |result| {
                            future::ready(ResultsDone::from_results_inner(
                                result,
                                call_result,
                                pipeline_sender,
                            ))
                        })
                    })
                    .then(move |v| {
                        if let Some(f) = redirected_results_done_fulfiller {
                            match v {
                                Ok(r) => drop(f.send(Ok(Response::redirected(r.clone())))),
                                Err(e) => drop(f.send(Err(e))),
                            }
                        }
                        Promise::ok(())
                    });

                let fork = promise.shared();
                pipeline.drive(fork.clone());

                {
                    let slots = &mut connection_state.answers.borrow_mut().slots;
                    let Some(answer) = slots.get_mut(&question_id) else {
                        unreachable!()
                    };
                    answer.pipeline = Some(Box::new(pipeline));
                    if redirect_results {
                        answer.redirected_results = redirected_results_done_promise;
                        // More to do here?
                    } else {
                        answer.call_completion_promise =
                            Some(connection_state.eagerly_evaluate(fork));
                    }
                }
            }
            Ok(message::Return(oret)) => {
                let ret = oret?;
                let question_id = ret.get_answer_id();

                let mut questions = connection_state.questions.borrow_mut();
                match questions.find(question_id) {
                    Some(ref mut question) => {
                        question.is_awaiting_return = false;
                        if ret.get_no_finish_needed() {
                            question.skip_finish = true;
                        }
                        match question.self_ref {
                            Some(ref question_ref) => match ret.which()? {
                                return_::Results(results) => {
                                    let cap_table = Self::receive_caps(
                                        &connection_state,
                                        results?.get_cap_table()?,
                                    )?;

                                    let question_ref =
                                        question_ref.upgrade().expect("dangling question ref?");
                                    let response = Response::new(
                                        connection_state.clone(),
                                        question_ref.clone(),
                                        message,
                                        cap_table,
                                    );
                                    question_ref.borrow_mut().fulfill(Promise::ok(response));
                                }
                                return_::Exception(e) => {
                                    let tmp =
                                        question_ref.upgrade().expect("dangling question ref?");
                                    tmp.borrow_mut().reject(remote_exception_to_error(e?));
                                }
                                return_::Canceled(_) => {
                                    Self::send_unimplemented(&connection_state, message.as_ref())?;
                                }
                                return_::ResultsSentElsewhere(_) => {
                                    Self::send_unimplemented(&connection_state, message.as_ref())?;
                                }
                                return_::TakeFromOtherQuestion(id) => {
                                    if let Some(answer) =
                                        connection_state.answers.borrow_mut().slots.get_mut(&id)
                                    {
                                        if let Some(res) = answer.redirected_results.take() {
                                            let tmp = question_ref
                                                .upgrade()
                                                .expect("dangling question ref?");
                                            tmp.borrow_mut().fulfill(res);
                                        } else {
                                            return Err(Error::failed("return.takeFromOtherQuestion referenced a call that \
                                                     did not use sendResultsTo.yourself.".to_string()));
                                        }
                                    } else {
                                        return Err(Error::failed(
                                            "return.takeFromOtherQuestion had invalid answer ID."
                                                .to_string(),
                                        ));
                                    }
                                }
                                return_::AcceptFromThirdParty(_) => {
                                    drop(questions);
                                    Self::send_unimplemented(&connection_state, message.as_ref())?;
                                }
                            },
                            None => {
                                if let return_::TakeFromOtherQuestion(_) = ret.which()? {
                                    return Self::send_unimplemented(
                                        &connection_state,
                                        message.as_ref(),
                                    );
                                }
                                // Looks like this question was canceled earlier, so `Finish`
                                // was already sent, with `releaseResultCaps` set true so that
                                // we don't have to release them here. We can go ahead and
                                // delete it from the table.
                                questions.erase(question_id);
                            }
                        }
                    }
                    None => {
                        return Err(Error::failed(format!(
                            "Invalid question ID in Return message: {question_id}"
                        )));
                    }
                }
            }
            Ok(message::Finish(finish)) => Self::handle_finish(&connection_state, finish?)?,
            Ok(message::Resolve(resolve)) => Self::handle_resolve(&connection_state, resolve?)?,
            Ok(message::Release(release)) => {
                let release = release?;
                connection_state.release_export(release.get_id(), release.get_reference_count())?;
            }
            Ok(message::Disembargo(disembargo)) => {
                Self::handle_disembargo(&connection_state, disembargo?)?
            }
            Ok(
                message::Provide(_)
                | message::Accept(_)
                | message::Join(_)
                | message::ObsoleteSave(_)
                | message::ObsoleteDelete(_),
            )
            | Err(::capnp::NotInSchema(_)) => {
                Self::send_unimplemented(&connection_state, message.as_ref())?;
            }
        }
        Ok(())
    }

    fn answer_has_sent_return(&self, id: AnswerId, result_exports: Vec<ExportId>) {
        let answers_slots = &mut self.answers.borrow_mut().slots;
        let hash_map::Entry::Occupied(mut entry) = answers_slots.entry(id) else {
            unreachable!()
        };
        let a = entry.get_mut();
        a.return_has_been_sent = true;
        if a.received_finish.get() {
            entry.remove();
        } else {
            a.result_exports = result_exports;
        }
    }

    fn release_export(&self, id: ExportId, refcount: u32) -> ::capnp::Result<()> {
        let mut exports = self.exports.borrow_mut();
        let Some(e) = exports.find(id) else {
            return Err(Error::failed(
                "Tried to release invalid export ID.".to_string(),
            ));
        };
        if refcount > e.refcount {
            return Err(Error::failed(
                "Tried to drop export's refcount below zero.".to_string(),
            ));
        }
        e.refcount -= refcount;
        if e.refcount == 0 {
            if let Some(client_ptr) = e.canonical {
                self.exports_by_cap.borrow_mut().remove(&client_ptr);
            }
            exports.erase(id);
        }
        Ok(())
    }

    fn release_exports(&self, exports: &[ExportId]) -> ::capnp::Result<()> {
        for &export_id in exports {
            self.release_export(export_id, 1)?;
        }
        Ok(())
    }

    fn get_brand(&self) -> usize {
        self as *const _ as usize
    }

    fn get_message_target(
        &self,
        target: message_target::Reader,
    ) -> ::capnp::Result<Box<dyn ClientHook>> {
        match target.which()? {
            message_target::ImportedCap(export_id) => {
                match self.exports.borrow().slots.get(export_id as usize) {
                    Some(Some(exp)) => Ok(exp.client_hook.clone()),
                    _ => Err(Error::failed(
                        "Message target is not a current export ID.".to_string(),
                    )),
                }
            }
            message_target::PromisedAnswer(promised_answer) => {
                let promised_answer = promised_answer?;
                let question_id = promised_answer.get_question_id();

                let pipeline = match self.answers.borrow().slots.get(&question_id) {
                    None => Box::new(broken::Pipeline::new(Error::failed(
                        "Pipeline call on a request that returned no capabilities or was already closed.".to_string(),
                    ))) as Box<dyn PipelineHook>,
                    Some(base) => {
                        match base.pipeline {
                            Some(ref pipeline) => pipeline.add_ref(),
                            None => Box::new(broken::Pipeline::new(Error::failed(
                                "Pipeline call on a request that returned not capabilities or was \
                                 already closed."
                                    .to_string(),
                            ))) as Box<dyn PipelineHook>,
                        }
                    }
                };
                let ops = to_pipeline_ops(promised_answer.get_transform()?)?;
                Ok(pipeline.get_pipelined_cap(&ops))
            }
        }
    }

    /// If calls to the given capability should pass over this connection, fill in `target`
    /// appropriately for such a call and return None. Otherwise, return a `ClientHook` to which
    /// the call should be forwarded; the caller should then delegate the call to that `ClientHook`.
    ///
    /// The main case where this ends up returning Some(_) is if `cap` is a promise that has
    /// recently resolved. The application might have started building a request before the promise
    /// resolved, and so the request may have been built on the assumption that it would be sent over
    /// this network connection, but then the promise resolved to point somewhere else before the
    /// request was sent. Now the request has to be redirected to the new target instead.
    fn write_target(
        &self,
        cap: &dyn ClientHook,
        target: message_target::Builder,
    ) -> Option<Box<dyn ClientHook>> {
        if cap.get_brand() == self.get_brand() {
            match Client::from_ptr(cap.get_ptr(), self) {
                Some(c) => c.write_target(target),
                None => unreachable!(),
            }
        } else {
            Some(cap.add_ref())
        }
    }

    /// If the given client just wraps some other client -- even if it is only *temporarily*
    /// wrapping that other client -- returns a reference to the other client, transitively.
    /// Otherwise, returns a new reference to *this.
    fn get_innermost_client(&self, mut client: Box<dyn ClientHook>) -> Box<dyn ClientHook> {
        while let Some(inner) = client.get_resolved() {
            client = inner;
        }
        if client.get_brand() == self.get_brand() {
            let ptr = client.get_ptr();
            match self.client_downcast_map.borrow().get(&ptr) {
                Some(c) => Box::new(c.upgrade().expect("dangling client?")),
                None => unreachable!(),
            }
        } else {
            client
        }
    }

    /// Implements exporting of a promise.  The promise has been exported under the given ID, and is
    /// to eventually resolve to the ClientHook produced by `promise`.  This method waits for that
    /// resolve to happen and then sends the appropriate `Resolve` message to the peer.
    #[allow(clippy::await_holding_refcell_ref)] // https://github.com/rust-lang/rust-clippy/issues/6353
    fn resolve_exported_promise(
        state: &Rc<Self>,
        export_id: ExportId,
        promise: Promise<Box<dyn ClientHook>, Error>,
    ) -> Promise<(), Error> {
        let weak_connection_state = Rc::downgrade(state);
        state.eagerly_evaluate(Promise::from_future(async move {
            let resolution_result = promise.await;
            let connection_state = weak_connection_state
                .upgrade()
                .expect("dangling connection state?");

            match resolution_result {
                Ok(resolution) => {
                    let resolution = connection_state.get_innermost_client(resolution);
                    let brand = resolution.get_brand();
                    let replacement = if brand != connection_state.get_brand() {
                        resolution.when_more_resolved()
                    } else {
                        None
                    };
                    let ptr = resolution.get_ptr();
                    let new_hook = resolution.clone();
                    connection_state.check_connected()?;

                    // Prepare hooks before borrowing either table, and retain the
                    // old hook until both table borrows have been released.
                    let mut exports = connection_state.exports.borrow_mut();
                    let Some(exp) = exports.find(export_id) else {
                        return Err(Error::failed("export table entry not found".to_string()));
                    };
                    let mut exports_by_cap = connection_state.exports_by_cap.borrow_mut();
                    if let Some(old_ptr) = exp.canonical.take() {
                        exports_by_cap.remove(&old_ptr);
                    }
                    let old_hook = mem::replace(&mut exp.client_hook, new_hook);
                    let reuse = replacement.is_some() && !exports_by_cap.contains_key(&ptr);
                    if reuse {
                        // A new local promise can reuse this export; other
                        // resolutions need a distinct descriptor below.
                        exports_by_cap.insert(ptr, export_id);
                        exp.canonical = Some(ptr);
                    }
                    drop(exports_by_cap);
                    drop(exports);
                    drop(old_hook);
                    connection_state.check_connected()?;
                    if reuse {
                        return Self::resolve_exported_promise(
                            &connection_state,
                            export_id,
                            replacement.unwrap(),
                        )
                        .await;
                    }
                    drop(replacement);

                    // OK, we have to send a `Resolve` message.
                    let mut message = connection_state.new_outgoing_message(15)?;
                    {
                        let root: message::Builder = message.get_body()?.get_as()?;
                        let mut resolve = root.init_resolve();
                        resolve.set_promise_id(export_id);
                        let _export = Self::write_descriptor(
                            &connection_state,
                            resolution,
                            resolve.init_cap(),
                        )?;
                    }
                    let _ = message.send();
                    Ok(())
                }
                Err(e) => {
                    // send error resolution
                    let mut message = connection_state.new_outgoing_message(15)?;
                    {
                        let root: message::Builder = message.get_body()?.get_as()?;
                        let mut resolve = root.init_resolve();
                        resolve.set_promise_id(export_id);
                        from_error(&e, resolve.init_exception());
                    }
                    let _ = message.send();
                    Ok(())
                }
            }
        }))
    }

    fn write_descriptor(
        state: &Rc<Self>,
        mut inner: Box<dyn ClientHook>,
        mut descriptor: cap_descriptor::Builder,
    ) -> ::capnp::Result<Option<ExportId>> {
        state.check_connected()?;
        // These hooks, including add_ref() through clone(), can run application
        // code. Prepare everything before admitting an export or a resolver.
        while let Some(resolved) = inner.get_resolved() {
            inner = resolved;
        }
        if inner.get_brand() == state.get_brand() {
            let ptr = inner.get_ptr();
            state.check_connected()?;
            if let Some(c) = Client::from_ptr(ptr, state) {
                return c.write_descriptor(descriptor);
            }
            // Preserve the duplicate-import fallback: a hook whose downcast
            // entry disappeared can still be exported as a foreign capability.
        }
        let ptr = inner.get_ptr();
        state.check_connected()?;
        let existing = state.exports_by_cap.borrow().get(&ptr).copied();
        if let Some(export_id) = existing {
            descriptor.set_sender_hosted(export_id);
            state.exports.borrow_mut().find(export_id).unwrap().refcount += 1;
            return Ok(Some(export_id));
        }

        let resolution = inner.when_more_resolved();
        state.check_connected()?;
        // Preparation may also have reentered while still connected and
        // exported this same capability. Reuse that canonical entry.
        let existing = state.exports_by_cap.borrow().get(&ptr).copied();
        if let Some(export_id) = existing {
            descriptor.set_sender_hosted(export_id);
            state.exports.borrow_mut().find(export_id).unwrap().refcount += 1;
            return Ok(Some(export_id));
        }
        let mut exp = Export::new(inner);
        exp.canonical = Some(ptr);
        let export_id = state.exports.borrow_mut().push(exp);
        state.exports_by_cap.borrow_mut().insert(ptr, export_id);
        match resolution {
            Some(wrapped) => {
                let resolve_op = Self::resolve_exported_promise(state, export_id, wrapped);
                state
                    .exports
                    .borrow_mut()
                    .find(export_id)
                    .unwrap()
                    .resolve_op = resolve_op;
                descriptor.set_sender_promise(export_id);
            }
            None => descriptor.set_sender_hosted(export_id),
        }
        Ok(Some(export_id))
    }

    fn write_descriptors(
        state: &Rc<Self>,
        cap_table: &[Option<Box<dyn ClientHook>>],
        payload: payload::Builder,
    ) -> capnp::Result<Vec<ExportId>> {
        let mut cap_table_builder = payload.init_cap_table(cap_table.len() as u32);
        let mut exports = Vec::new();
        for (idx, value) in cap_table.iter().enumerate() {
            match value {
                Some(cap) => {
                    if let Some(export_id) = Self::write_descriptor(
                        state,
                        cap.clone(),
                        cap_table_builder.reborrow().get(idx as u32),
                    )? {
                        exports.push(export_id);
                    }
                }
                None => {
                    cap_table_builder.reborrow().get(idx as u32).set_none(());
                }
            }
        }
        Ok(exports)
    }

    fn import(state: &Rc<Self>, import_id: ImportId, is_promise: bool) -> Box<dyn ClientHook> {
        let import_client = {
            match state.imports.borrow_mut().slots.entry(import_id) {
                hash_map::Entry::Occupied(occ) => occ
                    .get()
                    .import_client
                    .upgrade()
                    .expect("dangling ref to import client?"),
                hash_map::Entry::Vacant(v) => {
                    let import_client = ImportClient::new(state, import_id);
                    v.insert(Import::new(&import_client));
                    import_client
                }
            }
        };

        // We just received a copy of this import ID, so the remote refcount has gone up.
        import_client.borrow_mut().add_remote_ref();

        let mut tmp = state.imports.borrow_mut();
        let Some(import) = tmp.slots.get_mut(&import_id) else {
            unreachable!()
        };

        if is_promise {
            // We need to construct a PromiseClient around this import, if we haven't already.
            match &import.app_client {
                Some(c) => {
                    // Use the existing one.
                    Box::new(c.upgrade().expect("dangling client ref?"))
                }
                None => {
                    // Create a promise for this import's resolution.

                    let client: Box<Client<VatId>> = Box::new(import_client.into());
                    let client: Box<dyn ClientHook> = client;

                    // Here the C++ implementation does something like:
                    // ```
                    //   // Make sure the import is not destroyed while this promise exists.
                    //   let promise = promise.attach(client.add_ref());
                    // ```
                    // However, as far as I can tell that is unnecessary, because the
                    // PromiseClient holds `client` until it resolves, after which point
                    // there is no reason to keep the import alive.

                    let client = PromiseClient::new(state, client, Some(import_id));

                    import.promise_client_to_resolve = Some(Rc::downgrade(&client));
                    let client: Box<Client<VatId>> = Box::new(client.into());
                    import.app_client = Some(client.downgrade());
                    client
                }
            }
        } else {
            // Reuse the existing wrapper `Client` if one is still alive,
            // mirroring the promise branch above. Unconditionally creating a
            // new wrapper for an already-imported cap overwrites the
            // `client_downcast_map` entry (keyed by the shared inner
            // `ImportClient` pointer); when the newer wrapper is dropped
            // while an older one is still held by the application, the map's
            // weak reference dies and a later `write_descriptor` of the older
            // wrapper hits `Client::from_ptr() == None`.
            match import.app_client.as_ref().and_then(|c| c.upgrade()) {
                Some(c) => Box::new(c),
                None => {
                    let client: Box<Client<VatId>> = Box::new(import_client.into());
                    import.app_client = Some(client.downgrade());
                    client
                }
            }
        }
    }

    fn receive_cap(
        state: &Rc<Self>,
        descriptor: cap_descriptor::Reader,
    ) -> ::capnp::Result<Option<Box<dyn ClientHook>>> {
        match descriptor.which()? {
            cap_descriptor::None(()) => Ok(None),
            cap_descriptor::SenderHosted(sender_hosted) => {
                Ok(Some(Self::import(state, sender_hosted, false)))
            }
            cap_descriptor::SenderPromise(sender_promise) => {
                Ok(Some(Self::import(state, sender_promise, true)))
            }
            cap_descriptor::ReceiverHosted(receiver_hosted) => {
                if let Some(exp) = state.exports.borrow_mut().find(receiver_hosted) {
                    Ok(Some(exp.client_hook.add_ref()))
                } else {
                    Ok(Some(broken::new_cap(Error::failed(
                        "invalid 'receiverHosted' export ID".to_string(),
                    ))))
                }
            }
            cap_descriptor::ReceiverAnswer(receiver_answer) => {
                let promised_answer = receiver_answer?;
                let question_id = promised_answer.get_question_id();
                if let Some(answer) = state.answers.borrow().slots.get(&question_id) {
                    if let Some(ref pipeline) = answer.pipeline {
                        let ops = to_pipeline_ops(promised_answer.get_transform()?)?;
                        return Ok(Some(pipeline.get_pipelined_cap(&ops)));
                    }
                }
                Ok(Some(broken::new_cap(Error::failed(
                    "invalid 'receiver answer'".to_string(),
                ))))
            }
            cap_descriptor::ThirdPartyHosted(_third_party_hosted) => Err(Error::unimplemented(
                "ThirdPartyHosted caps are not supported.".to_string(),
            )),
        }
    }

    fn receive_caps(
        state: &Rc<Self>,
        cap_table: ::capnp::struct_list::Reader<cap_descriptor::Owned>,
    ) -> ::capnp::Result<Vec<Option<Box<dyn ClientHook>>>> {
        let mut result = Vec::new();
        for idx in 0..cap_table.len() {
            result.push(Self::receive_cap(state, cap_table.get(idx))?);
        }
        Ok(result)
    }
}

enum DisconnectorState {
    New,
    Disconnecting(future::Shared<Promise<(), Error>>),
    Disconnected,
}

/// A `Future` that can be run to disconnect an `RpcSystem`'s ConnectionState and wait for it to be closed.
pub struct Disconnector<VatId>
where
    VatId: 'static,
{
    connection_state: Rc<RefCell<Option<Rc<ConnectionState<VatId>>>>>,
    state: DisconnectorState,
}

impl<VatId> Disconnector<VatId> {
    pub fn new(connection_state: Rc<RefCell<Option<Rc<ConnectionState<VatId>>>>>) -> Self {
        Self {
            connection_state,
            state: DisconnectorState::New,
        }
    }
    fn disconnect(&self) -> Option<future::Shared<Promise<(), Error>>> {
        let state = self.connection_state.borrow().clone();
        if let Some(state) = state {
            state.disconnect(::capnp::Error::disconnected(
                "client requested disconnect".to_owned(),
            ));
            state.disconnect_promise.borrow().clone()
        } else {
            None
        }
    }
}

impl<VatId> Future for Disconnector<VatId>
where
    VatId: 'static,
{
    type Output = Result<(), capnp::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = &mut *self;
        loop {
            match this.state {
                DisconnectorState::New => {
                    this.state = match this.disconnect() {
                        Some(shutdown_promise) => {
                            DisconnectorState::Disconnecting(shutdown_promise)
                        }
                        None => DisconnectorState::Disconnected,
                    };
                }
                DisconnectorState::Disconnecting(ref mut shutdown_promise) => {
                    let result = futures::ready!(Pin::new(shutdown_promise).poll(cx));
                    this.state = DisconnectorState::Disconnected;
                    return Poll::Ready(result);
                }
                DisconnectorState::Disconnected => return Poll::Ready(Ok(())),
            }
        }
    }
}

struct ResponseState<VatId>
where
    VatId: 'static,
{
    _connection_state: Rc<ConnectionState<VatId>>,
    message: Box<dyn crate::IncomingMessage>,
    cap_table: Vec<Option<Box<dyn ClientHook>>>,
    _question_ref: Rc<RefCell<QuestionRef<VatId>>>,
}

enum ResponseVariant<VatId>
where
    VatId: 'static,
{
    Rpc(ResponseState<VatId>),
    LocallyRedirected(Box<dyn ResultsDoneHook>),
}

struct Response<VatId>
where
    VatId: 'static,
{
    variant: Rc<ResponseVariant<VatId>>,
}

impl<VatId> Response<VatId> {
    fn new(
        connection_state: Rc<ConnectionState<VatId>>,
        question_ref: Rc<RefCell<QuestionRef<VatId>>>,
        message: Box<dyn crate::IncomingMessage>,
        cap_table_array: Vec<Option<Box<dyn ClientHook>>>,
    ) -> Self {
        Self {
            variant: Rc::new(ResponseVariant::Rpc(ResponseState {
                _connection_state: connection_state,
                message,
                cap_table: cap_table_array,
                _question_ref: question_ref,
            })),
        }
    }
    fn redirected(results_done: Box<dyn ResultsDoneHook>) -> Self {
        Self {
            variant: Rc::new(ResponseVariant::LocallyRedirected(results_done)),
        }
    }
}

impl<VatId> Clone for Response<VatId> {
    fn clone(&self) -> Self {
        Self {
            variant: self.variant.clone(),
        }
    }
}

impl<VatId> ResponseHook for Response<VatId> {
    fn get(&self) -> ::capnp::Result<any_pointer::Reader<'_>> {
        match *self.variant {
            ResponseVariant::Rpc(ref state) => {
                match state
                    .message
                    .get_body()?
                    .get_as::<message::Reader>()?
                    .which()?
                {
                    message::Return(Ok(ret)) => match ret.which()? {
                        return_::Results(Ok(mut payload)) => {
                            use ::capnp::traits::Imbue;
                            payload.imbue(&state.cap_table);
                            Ok(payload.get_content())
                        }
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                }
            }
            ResponseVariant::LocallyRedirected(ref results_done) => results_done.get(),
        }
    }
}

struct Request<VatId>
where
    VatId: 'static,
{
    connection_state: Rc<ConnectionState<VatId>>,
    target: Client<VatId>,
    message: Box<dyn crate::OutgoingMessage>,
    cap_table: Vec<Option<Box<dyn ClientHook>>>,
}

fn get_call(message: &mut Box<dyn crate::OutgoingMessage>) -> ::capnp::Result<call::Builder<'_>> {
    let message_root: message::Builder = message.get_body()?.get_as()?;
    match message_root.which()? {
        message::Call(call) => call,
        _ => {
            unimplemented!()
        }
    }
}

impl<VatId> Request<VatId>
where
    VatId: 'static,
{
    fn new(
        connection_state: Rc<ConnectionState<VatId>>,
        _size_hint: Option<::capnp::MessageSize>,
        target: Client<VatId>,
    ) -> ::capnp::Result<Self> {
        let message = connection_state.new_outgoing_message(1024)?;
        Ok(Self {
            connection_state,
            target,
            message,
            cap_table: Vec::new(),
        })
    }

    fn init_call(&mut self) -> call::Builder<'_> {
        let message_root: message::Builder = self.message.get_body().unwrap().get_as().unwrap();
        message_root.init_call()
    }

    fn send_internal(
        connection_state: &Rc<ConnectionState<VatId>>,
        mut message: Box<dyn crate::OutgoingMessage>,
        cap_table: &[Option<Box<dyn ClientHook>>],
        is_tail_call: bool,
    ) -> capnp::Result<(
        Rc<RefCell<QuestionRef<VatId>>>,
        Promise<Response<VatId>, Error>,
    )> {
        // Build the cap table.
        let exports = ConnectionState::write_descriptors(
            connection_state,
            cap_table,
            get_call(&mut message).unwrap().get_params().unwrap(),
        )?;
        // A temporary descriptor hook can disconnect even as it is dropped.
        // Disconnect already releases all exports prepared by this request.
        connection_state.check_connected()?;

        // Init the question table.  Do this after writing descriptors to avoid interference.
        let mut question = Question::<VatId>::new();
        question.is_awaiting_return = true;
        question.param_exports = exports;
        question.is_tail_call = is_tail_call;

        let question_id = connection_state.questions.borrow_mut().push(question);
        {
            let mut call_builder: call::Builder = get_call(&mut message).unwrap();
            // Finish and send.
            call_builder.reborrow().set_question_id(question_id);
            if is_tail_call {
                call_builder.get_send_results_to().set_yourself(());
            }
        }
        let _ = message.send();
        // Make the result promise.
        let (fulfiller, promise) = oneshot::channel::<Promise<Response<VatId>, Error>>();
        let promise = promise.map_err(crate::canceled_to_error).and_then(|x| x);
        let question_ref = Rc::new(RefCell::new(QuestionRef::new(
            connection_state.clone(),
            question_id,
            fulfiller,
        )));

        match connection_state.questions.borrow_mut().slots[question_id as usize] {
            Some(ref mut q) => {
                q.self_ref = Some(Rc::downgrade(&question_ref));
            }
            None => unreachable!(),
        }

        let promise = promise.attach(question_ref.clone());
        let promise2 = Promise::from_future(promise);

        Ok((question_ref, promise2))
    }

    fn send_streaming_internal(
        connection_state: &Rc<ConnectionState<VatId>>,
        mut message: Box<dyn crate::OutgoingMessage>,
        cap_table: &[Option<Box<dyn ClientHook>>],
        flow: Rc<RefCell<Option<Box<dyn crate::FlowController>>>>,
    ) -> Promise<(), Error> {
        // Build the cap table.
        let exports = pry!(ConnectionState::write_descriptors(
            connection_state,
            cap_table,
            get_call(&mut message).unwrap().get_params().unwrap(),
        ));
        pry!(connection_state.check_connected());

        // Init the question table.  Do this after writing descriptors to avoid interference.
        let mut question = Question::<VatId>::new();
        question.is_awaiting_return = true;
        question.param_exports = exports;
        question.is_tail_call = false;

        let question_id = connection_state.questions.borrow_mut().push(question);
        {
            let mut call_builder: call::Builder = get_call(&mut message).unwrap();
            call_builder.reborrow().set_question_id(question_id);
        }

        // Make the result promise.
        let (fulfiller, promise) = oneshot::channel::<Promise<Response<VatId>, Error>>();
        let promise = promise.map_err(crate::canceled_to_error).and_then(|x| x);
        let question_ref = Rc::new(RefCell::new(QuestionRef::new(
            connection_state.clone(),
            question_id,
            fulfiller,
        )));

        match connection_state.questions.borrow_mut().slots[question_id as usize] {
            Some(ref mut q) => {
                q.self_ref = Some(Rc::downgrade(&question_ref));
            }
            None => unreachable!(),
        }
        let promise = promise.attach(question_ref.clone());

        let mut flow = flow.borrow_mut();
        if flow.is_none() {
            match connection_state.connection.borrow_mut().as_mut() {
                Err(error) => return Promise::err(error.clone()),
                Ok(connection) => {
                    let (s, p) = connection.new_stream();
                    connection_state.add_task(p);
                    *flow = Some(s);
                }
            };
        }
        let Some(ref mut flow) = *flow else {
            unreachable!()
        };
        flow.send(
            message,
            Promise::from_future(async move {
                let _ = promise.await?;
                Ok(())
            }),
        )
    }
}

impl<VatId> RequestHook for Request<VatId> {
    fn get(&mut self) -> any_pointer::Builder<'_> {
        use ::capnp::traits::ImbueMut;
        let mut builder = get_call(&mut self.message)
            .unwrap()
            .get_params()
            .unwrap()
            .get_content();
        builder.imbue_mut(&mut self.cap_table);
        builder
    }
    fn get_brand<'a>(&self) -> usize {
        self.connection_state.get_brand()
    }
    fn send(self: Box<Self>) -> ::capnp::capability::RemotePromise<any_pointer::Owned> {
        let tmp = *self;
        let Self {
            connection_state,
            target,
            mut message,
            cap_table,
        } = tmp;
        // A request can outlive the connection it was constructed on. Check
        // again at send admission, before writing descriptors or questions.
        let terminal = connection_state.connection.borrow().as_ref().err().cloned();
        if let Some(error) = terminal {
            return ::capnp::capability::RemotePromise {
                promise: Promise::err(error.clone()),
                pipeline: any_pointer::Pipeline::new(Box::new(broken::Pipeline::new(error))),
            };
        }
        let write_target_result = {
            let call_builder: call::Builder = get_call(&mut message).unwrap();
            target.write_target(call_builder.get_target().unwrap())
        };
        if let Some(redirect) = write_target_result {
            // Whoops, this capability has been redirected while we were building the request!
            // We'll have to make a new request and do a copy.  Ick.
            let mut call_builder: call::Builder = get_call(&mut message).unwrap();
            let mut replacement = redirect.new_call(
                call_builder.reborrow().get_interface_id(),
                call_builder.reborrow().get_method_id(),
                None,
            );

            replacement
                .set(
                    call_builder
                        .get_params()
                        .unwrap()
                        .get_content()
                        .into_reader(),
                )
                .unwrap();
            return replacement.send();
        }
        let (question_ref, promise) =
            match Self::send_internal(&connection_state, message, &cap_table, false) {
                Ok(result) => result,
                Err(error) => {
                    return ::capnp::capability::RemotePromise {
                        promise: Promise::err(error.clone()),
                        pipeline: any_pointer::Pipeline::new(Box::new(broken::Pipeline::new(
                            error,
                        ))),
                    }
                }
            };
        let forked_promise1 = promise.shared();
        let forked_promise2 = forked_promise1.clone();

        // The pipeline must get notified of resolution before the app does to maintain ordering.
        let pipeline = Pipeline::new(
            &connection_state,
            question_ref,
            Some(Promise::from_future(forked_promise1)),
        );

        let resolved = pipeline.when_resolved();

        let forked_promise2 = resolved.map(|_| Ok(())).and_then(|()| forked_promise2);

        let app_promise = Promise::from_future(
            forked_promise2
                .map_ok(|response| ::capnp::capability::Response::new(Box::new(response))),
        );

        ::capnp::capability::RemotePromise {
            promise: app_promise,
            pipeline: any_pointer::Pipeline::new(Box::new(pipeline)),
        }
    }
    fn send_streaming(self: Box<Self>) -> Promise<(), Error> {
        let tmp = *self;
        let Self {
            connection_state,
            target,
            mut message,
            cap_table,
        } = tmp;
        // Retained streaming requests must not recreate drained exports or
        // questions, even when their flow controller already exists.
        let terminal = connection_state.connection.borrow().as_ref().err().cloned();
        if let Some(error) = terminal {
            return Promise::err(error);
        }
        let write_target_result = {
            let call_builder: call::Builder = get_call(&mut message).unwrap();
            target.write_target(call_builder.get_target().unwrap())
        };
        if let Some(redirect) = write_target_result {
            // Whoops, this capability has been redirected while we were building the request!
            // We'll have to make a new request and do a copy.  Ick.
            let mut call_builder: call::Builder = get_call(&mut message).unwrap();
            let mut replacement = redirect.new_call(
                call_builder.reborrow().get_interface_id(),
                call_builder.reborrow().get_method_id(),
                None,
            );

            replacement
                .set(
                    call_builder
                        .get_params()
                        .unwrap()
                        .get_content()
                        .into_reader(),
                )
                .unwrap();
            return replacement.hook.send_streaming();
        }
        Self::send_streaming_internal(
            &connection_state,
            message,
            &cap_table,
            target.flow_controller,
        )
    }
    fn tail_send(self: Box<Self>) -> Option<(u32, Promise<(), Error>, Box<dyn PipelineHook>)> {
        let tmp = *self;
        let Self {
            connection_state,
            target,
            mut message,
            cap_table,
        } = tmp;

        if connection_state.connection.borrow().is_err() {
            // Disconnected; fall back to a regular send() which will fail appropriately.
            return None;
        }

        let write_target_result = {
            let call_builder: crate::rpc_capnp::call::Builder = get_call(&mut message).unwrap();
            target.write_target(call_builder.get_target().unwrap())
        };

        let (question_ref, promise) = match write_target_result {
            Some(_redirect) => {
                return None;
            }
            None => Self::send_internal(&connection_state, message, &cap_table, true).ok()?,
        };

        let promise = promise.map_ok(|_response| {
            // Response should be null if `Return` handling code is correct.

            unimplemented!()
        });

        let question_id = question_ref.borrow().id;
        let pipeline = Pipeline::never_done(connection_state, question_ref);

        Some((
            question_id,
            Promise::from_future(promise),
            Box::new(pipeline),
        ))
    }
}

enum PipelineVariant<VatId>
where
    VatId: 'static,
{
    Waiting(Rc<RefCell<QuestionRef<VatId>>>),
    Resolved(Response<VatId>),
    Broken(Error),
}

struct PipelineState<VatId>
where
    VatId: 'static,
{
    variant: PipelineVariant<VatId>,
    redirect_later: Option<RefCell<futures::future::Shared<Promise<Response<VatId>, Error>>>>,
    connection_state: Rc<ConnectionState<VatId>>,

    #[allow(dead_code)]
    resolve_self_promise: Promise<(), Error>,

    promise_clients_to_resolve: RefCell<
        crate::sender_queue::SenderQueue<
            (Weak<RefCell<PromiseClient<VatId>>>, Vec<PipelineOp>),
            (),
        >,
    >,
    resolution_waiters: crate::sender_queue::SenderQueue<(), ()>,
}

impl<VatId> PipelineState<VatId>
where
    VatId: 'static,
{
    fn resolve(state: &Rc<RefCell<Self>>, response: Result<Response<VatId>, Error>) {
        let (old_variant, to_resolve, waiters) = {
            let mut state = state.borrow_mut();
            if !matches!(state.variant, PipelineVariant::Waiting(_)) {
                // A terminal rejection may precede the scheduled resolver.
                return;
            }
            let new_variant = match response.clone() {
                Ok(r) => PipelineVariant::Resolved(r),
                Err(e) => PipelineVariant::Broken(e),
            };
            let old_variant = mem::replace(&mut state.variant, new_variant);
            let to_resolve = state.promise_clients_to_resolve.borrow_mut().drain();
            let waiters = state.resolution_waiters.drain();
            (old_variant, to_resolve, waiters)
        };
        for ((client, ops), _) in to_resolve {
            let resolved = match response.clone() {
                Ok(value) => value.get().and_then(|value| value.get_pipelined_cap(&ops)),
                Err(error) => Err(error),
            };
            if let Some(client) = client.upgrade() {
                client.borrow_mut().resolve(resolved);
            }
        }
        for (_, waiter) in waiters {
            let _ = waiter.send(());
        }
        // Dropping the question can reenter the connection's question table.
        drop(old_variant);
    }
}

struct Pipeline<VatId>
where
    VatId: 'static,
{
    state: Rc<RefCell<PipelineState<VatId>>>,
}

impl<VatId> Pipeline<VatId> {
    fn new(
        connection_state: &Rc<ConnectionState<VatId>>,
        question_ref: Rc<RefCell<QuestionRef<VatId>>>,
        redirect_later: Option<Promise<Response<VatId>, ::capnp::Error>>,
    ) -> Self {
        let state = Rc::new(RefCell::new(PipelineState {
            variant: PipelineVariant::Waiting(question_ref.clone()),
            connection_state: connection_state.clone(),
            redirect_later: None,
            resolve_self_promise: Promise::from_future(future::pending()),
            promise_clients_to_resolve: RefCell::new(crate::sender_queue::SenderQueue::new()),
            resolution_waiters: crate::sender_queue::SenderQueue::new(),
        }));
        question_ref
            .borrow_mut()
            .pipelines
            .push(Rc::downgrade(&state));
        if let Some(redirect_later_promise) = redirect_later {
            let fork = redirect_later_promise.shared();
            let this = Rc::downgrade(&state);
            let resolve_self_promise =
                connection_state.eagerly_evaluate(fork.clone().then(move |response| {
                    let Some(state) = this.upgrade() else {
                        return Promise::err(Error::failed("dangling reference to this".into()));
                    };
                    PipelineState::resolve(&state, response);
                    Promise::ok(())
                }));

            state.borrow_mut().resolve_self_promise = resolve_self_promise;
            state.borrow_mut().redirect_later = Some(RefCell::new(fork));
        }
        Self { state }
    }

    fn when_resolved(&self) -> Promise<(), Error> {
        self.state.borrow_mut().resolution_waiters.push(())
    }

    fn never_done(
        connection_state: Rc<ConnectionState<VatId>>,
        question_ref: Rc<RefCell<QuestionRef<VatId>>>,
    ) -> Self {
        let state = Rc::new(RefCell::new(PipelineState {
            variant: PipelineVariant::Waiting(question_ref.clone()),
            connection_state,
            redirect_later: None,
            resolve_self_promise: Promise::from_future(future::pending()),
            promise_clients_to_resolve: RefCell::new(crate::sender_queue::SenderQueue::new()),
            resolution_waiters: crate::sender_queue::SenderQueue::new(),
        }));
        question_ref
            .borrow_mut()
            .pipelines
            .push(Rc::downgrade(&state));

        Self { state }
    }
}

impl<VatId> PipelineHook for Pipeline<VatId> {
    fn add_ref(&self) -> Box<dyn PipelineHook> {
        Box::new(Self {
            state: self.state.clone(),
        })
    }
    fn get_pipelined_cap(&self, ops: &[PipelineOp]) -> Box<dyn ClientHook> {
        self.get_pipelined_cap_move(ops.into())
    }
    fn get_pipelined_cap_move(&self, ops: Vec<PipelineOp>) -> Box<dyn ClientHook> {
        match *self.state.borrow() {
            PipelineState {
                variant: PipelineVariant::Waiting(ref question_ref),
                ref connection_state,
                ref redirect_later,
                ref promise_clients_to_resolve,
                ..
            } => {
                // Wrap a PipelineClient in a PromiseClient.
                let pipeline_client =
                    PipelineClient::new(connection_state, question_ref.clone(), ops.clone());

                match redirect_later {
                    Some(_r) => {
                        let client: Client<VatId> = pipeline_client.into();
                        let promise_client =
                            PromiseClient::new(connection_state, Box::new(client), None);
                        promise_clients_to_resolve
                            .borrow_mut()
                            .push_detach((Rc::downgrade(&promise_client), ops));
                        let result: Client<VatId> = promise_client.into();
                        Box::new(result)
                    }
                    None => {
                        // Oh, this pipeline will never get redirected, so just return the PipelineClient.
                        let client: Client<VatId> = pipeline_client.into();
                        Box::new(client)
                    }
                }
            }
            PipelineState {
                variant: PipelineVariant::Resolved(ref response),
                ..
            } => response.get().unwrap().get_pipelined_cap(&ops[..]).unwrap(),
            PipelineState {
                variant: PipelineVariant::Broken(ref e),
                ..
            } => broken::new_cap(e.clone()),
        }
    }
}

pub(crate) struct Params {
    request: Box<dyn crate::IncomingMessage>,
    cap_table: Vec<Option<Box<dyn ClientHook>>>,
}

impl Params {
    fn new(
        request: Box<dyn crate::IncomingMessage>,
        cap_table: Vec<Option<Box<dyn ClientHook>>>,
    ) -> Self {
        Self { request, cap_table }
    }
}

impl ParamsHook for Params {
    fn get(&self) -> ::capnp::Result<any_pointer::Reader<'_>> {
        let root: message::Reader = self.request.get_body()?.get_as()?;
        let message::Call(call) = root.which()? else {
            unreachable!()
        };
        use ::capnp::traits::Imbue;
        let mut content = call?.get_params()?.get_content();
        content.imbue(&self.cap_table);
        Ok(content)
    }
}

enum ResultsVariant {
    Rpc(
        Box<dyn crate::OutgoingMessage>,
        Vec<Option<Box<dyn ClientHook>>>,
    ),
    LocallyRedirected(
        ::capnp::message::Builder<::capnp::message::HeapAllocator>,
        Vec<Option<Box<dyn ClientHook>>>,
    ),
}

struct ResultsInner<VatId>
where
    VatId: 'static,
{
    connection_state: Rc<ConnectionState<VatId>>,
    variant: Option<ResultsVariant>,
    redirect_results: bool,
    answer_id: AnswerId,
    finish_received: Rc<Cell<bool>>,
    pipeline_sender: Option<queued::PipelineInnerSender>,
}

impl<VatId> ResultsInner<VatId>
where
    VatId: 'static,
{
    fn ensure_initialized(&mut self) {
        let answer_id = self.answer_id;
        if self.variant.is_none() {
            match (
                self.redirect_results,
                self.connection_state.connection.borrow_mut().as_mut(),
            ) {
                (false, Ok(c)) => {
                    let mut message = c.new_outgoing_message(100); // size hint?

                    {
                        let root: message::Builder = message.get_body().unwrap().init_as();
                        let mut ret = root.init_return();
                        ret.set_answer_id(answer_id);
                        ret.set_release_param_caps(false);
                    }
                    self.variant = Some(ResultsVariant::Rpc(message, Vec::new()));
                }
                _ => {
                    self.variant = Some(ResultsVariant::LocallyRedirected(
                        ::capnp::message::Builder::new_default(),
                        Vec::new(),
                    ));
                }
            }
        }
    }
}

// This takes the place of both RpcCallContext and RpcServerResponse in capnproto-c++.
pub(crate) struct Results<VatId>
where
    VatId: 'static,
{
    inner: Option<ResultsInner<VatId>>,
    results_done_fulfiller: Option<oneshot::Sender<ResultsInner<VatId>>>,
}

impl<VatId> Results<VatId>
where
    VatId: 'static,
{
    fn new(
        connection_state: &Rc<ConnectionState<VatId>>,
        answer_id: AnswerId,
        redirect_results: bool,
        fulfiller: oneshot::Sender<ResultsInner<VatId>>,
        finish_received: Rc<Cell<bool>>,
        pipeline_sender: Option<queued::PipelineInnerSender>,
    ) -> Self {
        Self {
            inner: Some(ResultsInner {
                variant: None,
                connection_state: connection_state.clone(),
                redirect_results,
                answer_id,
                finish_received,
                pipeline_sender,
            }),
            results_done_fulfiller: Some(fulfiller),
        }
    }
}

impl<VatId> Drop for Results<VatId> {
    fn drop(&mut self) {
        match (self.inner.take(), self.results_done_fulfiller.take()) {
            (Some(inner), Some(fulfiller)) => {
                let _ = fulfiller.send(inner);
            }
            (None, None) => (),
            _ => unreachable!(),
        }
    }
}

impl<VatId> ResultsHook for Results<VatId> {
    fn get(&mut self) -> ::capnp::Result<any_pointer::Builder<'_>> {
        use ::capnp::traits::ImbueMut;
        let Some(ref mut inner) = self.inner else {
            unreachable!();
        };
        inner.ensure_initialized();
        match inner.variant {
            None => unreachable!(),
            Some(ResultsVariant::Rpc(ref mut message, ref mut cap_table)) => {
                let root: message::Builder = message.get_body()?.get_as()?;
                let message::Return(ret) = root.which()? else {
                    unreachable!();
                };
                let return_::Results(payload) = ret?.which()? else {
                    unreachable!()
                };
                let mut content = payload?.get_content();
                content.imbue_mut(cap_table);
                Ok(content)
            }
            Some(ResultsVariant::LocallyRedirected(ref mut message, ref mut cap_table)) => {
                let mut result: any_pointer::Builder = message.get_root()?;
                result.imbue_mut(cap_table);
                Ok(result)
            }
        }
    }

    fn set_pipeline(&mut self) -> ::capnp::Result<()> {
        use ::capnp::traits::ImbueMut;
        let root = self.get()?;
        let size = root.target_size()?;
        let mut message2 = capnp::message::Builder::new(
            capnp::message::HeapAllocator::new().first_segment_words(size.word_count as u32 + 1),
        );
        let mut root2: capnp::any_pointer::Builder = message2.init_root();
        let mut cap_table2 = vec![];
        root2.imbue_mut(&mut cap_table2);
        root2.set_as(root.into_reader())?;
        let hook =
            Box::new(local::ResultsDone::new(message2, cap_table2)) as Box<dyn ResultsDoneHook>;
        let Some(ref mut inner) = self.inner else {
            unreachable!();
        };
        let Some(sender) = inner.pipeline_sender.take() else {
            return Err(Error::failed("set_pipeline() called twice".into()));
        };
        sender.complete(Box::new(local::Pipeline::new(hook)));
        Ok(())
    }

    fn tail_call(self: Box<Self>, _request: Box<dyn RequestHook>) -> Promise<(), Error> {
        unimplemented!()
    }

    fn direct_tail_call(
        mut self: Box<Self>,
        request: Box<dyn RequestHook>,
    ) -> (Promise<(), Error>, Box<dyn PipelineHook>) {
        if let (Some(inner), Some(fulfiller)) =
            (self.inner.take(), self.results_done_fulfiller.take())
        {
            let state = inner.connection_state.clone();
            if request.get_brand() == state.get_brand() && !inner.redirect_results {
                // The tail call is headed towards the peer that called us in the first place, so we can
                // optimize out the return trip.
                if let Some((question_id, promise, pipeline)) = request.tail_send() {
                    let mut message = state.new_outgoing_message(100).expect("no connection?"); // size hint?

                    {
                        let root: message::Builder = message.get_body().unwrap().init_as();
                        let mut ret = root.init_return();
                        ret.set_answer_id(inner.answer_id);
                        ret.set_release_param_caps(false);
                        ret.set_take_from_other_question(question_id);
                    }
                    let _ = message.send();

                    // TODO cleanupanswertable

                    let _ = fulfiller.send(inner); // ??
                    return (promise, pipeline);
                }
                unimplemented!()
            } else {
                unimplemented!()
            }
        } else {
            unreachable!();
        }
    }

    fn allow_cancellation(&self) {
        unimplemented!()
    }
}

enum ResultsDoneVariant {
    Rpc(
        Rc<::capnp::message::Builder<::capnp::message::HeapAllocator>>,
        Vec<Option<Box<dyn ClientHook>>>,
    ),
    LocallyRedirected(
        ::capnp::message::Builder<::capnp::message::HeapAllocator>,
        Vec<Option<Box<dyn ClientHook>>>,
    ),
}

struct ResultsDone {
    inner: Rc<ResultsDoneVariant>,
}

impl ResultsDone {
    fn from_results_inner<VatId>(
        results_inner: Result<ResultsInner<VatId>, Error>,
        call_status: Result<(), Error>,
        pipeline_sender: queued::PipelineInnerSender,
    ) -> Result<Box<dyn ResultsDoneHook>, Error>
    where
        VatId: 'static,
    {
        match results_inner {
            Err(e) => {
                pipeline_sender.complete(Box::new(crate::broken::Pipeline::new(e.clone())));
                Err(e)
            }
            Ok(mut results_inner) => {
                results_inner.ensure_initialized();
                let ResultsInner {
                    connection_state,
                    variant,
                    answer_id,
                    finish_received,
                    ..
                } = results_inner;
                match variant {
                    None => unreachable!(),
                    Some(ResultsVariant::Rpc(mut message, cap_table)) => {
                        match (finish_received.get(), call_status) {
                            (true, _) => {
                                let hook = Box::new(Self::rpc(Rc::new(message.take()), cap_table))
                                    as Box<dyn ResultsDoneHook>;
                                pipeline_sender
                                    .complete(Box::new(local::Pipeline::new(hook.clone())));

                                // Send a Canceled return.
                                if let Ok(connection) =
                                    connection_state.connection.borrow_mut().as_mut()
                                {
                                    let mut message = connection.new_outgoing_message(10);
                                    {
                                        let root: message::Builder =
                                            message.get_body()?.get_as()?;
                                        let mut ret = root.init_return();
                                        ret.set_answer_id(answer_id);
                                        ret.set_release_param_caps(false);
                                        ret.set_canceled(());
                                    }
                                    let _ = message.send();
                                }

                                connection_state.answer_has_sent_return(answer_id, Vec::new());
                                Ok(hook)
                            }
                            (false, Ok(())) => {
                                let exports = {
                                    let root: message::Builder = message.get_body()?.get_as()?;
                                    let message::Return(Ok(mut ret)) = root.which()? else {
                                        unreachable!()
                                    };
                                    if cap_table.is_empty() {
                                        ret.set_no_finish_needed(true);
                                        finish_received.set(true);
                                    }
                                    let crate::rpc_capnp::return_::Results(Ok(payload)) =
                                        ret.which()?
                                    else {
                                        unreachable!()
                                    };
                                    match ConnectionState::write_descriptors(
                                        &connection_state,
                                        &cap_table,
                                        payload,
                                    )
                                    .and_then(|exports| {
                                        connection_state.check_connected()?;
                                        Ok(exports)
                                    }) {
                                        Ok(exports) => exports,
                                        Err(error) => {
                                            pipeline_sender.complete(Box::new(
                                                broken::Pipeline::new(error.clone()),
                                            ));
                                            return Err(error);
                                        }
                                    }
                                };

                                let (_promise, m) = message.send();
                                connection_state.answer_has_sent_return(answer_id, exports);
                                let hook =
                                    Box::new(Self::rpc(m, cap_table)) as Box<dyn ResultsDoneHook>;
                                pipeline_sender
                                    .complete(Box::new(local::Pipeline::new(hook.clone())));
                                Ok(hook)
                            }
                            (false, Err(e)) => {
                                // Send an error return.
                                if let Ok(connection) =
                                    connection_state.connection.borrow_mut().as_mut()
                                {
                                    let mut message = connection.new_outgoing_message(50); // XXX size hint
                                    {
                                        let root: message::Builder =
                                            message.get_body()?.get_as()?;
                                        let mut ret = root.init_return();
                                        ret.set_answer_id(answer_id);
                                        ret.set_release_param_caps(false);
                                        let mut exc = ret.init_exception();
                                        from_error(&e, exc.reborrow());
                                    }
                                    let _ = message.send();
                                }
                                connection_state.answer_has_sent_return(answer_id, Vec::new());

                                pipeline_sender
                                    .complete(Box::new(crate::broken::Pipeline::new(e.clone())));

                                Err(e)
                            }
                        }
                    }
                    Some(ResultsVariant::LocallyRedirected(results_done, cap_table)) => {
                        let hook = Box::new(Self::redirected(results_done, cap_table))
                            as Box<dyn ResultsDoneHook>;
                        pipeline_sender
                            .complete(Box::new(crate::local::Pipeline::new(hook.clone())));
                        Ok(hook)
                    }
                }
            }
        }
    }

    fn rpc(
        message: Rc<::capnp::message::Builder<::capnp::message::HeapAllocator>>,
        cap_table: Vec<Option<Box<dyn ClientHook>>>,
    ) -> Self {
        Self {
            inner: Rc::new(ResultsDoneVariant::Rpc(message, cap_table)),
        }
    }

    fn redirected(
        message: ::capnp::message::Builder<::capnp::message::HeapAllocator>,
        cap_table: Vec<Option<Box<dyn ClientHook>>>,
    ) -> Self {
        Self {
            inner: Rc::new(ResultsDoneVariant::LocallyRedirected(message, cap_table)),
        }
    }
}

impl ResultsDoneHook for ResultsDone {
    fn add_ref(&self) -> Box<dyn ResultsDoneHook> {
        Box::new(Self {
            inner: self.inner.clone(),
        })
    }
    fn get(&self) -> ::capnp::Result<any_pointer::Reader<'_>> {
        use ::capnp::traits::Imbue;
        match *self.inner {
            ResultsDoneVariant::Rpc(ref message, ref cap_table) => {
                let root: message::Reader = message.get_root_as_reader()?;
                let message::Return(ret) = root.which()? else {
                    unreachable!();
                };
                let crate::rpc_capnp::return_::Results(payload) = ret?.which()? else {
                    unreachable!();
                };
                let mut content = payload?.get_content();
                content.imbue(cap_table);
                Ok(content)
            }
            ResultsDoneVariant::LocallyRedirected(ref message, ref cap_table) => {
                let mut result: any_pointer::Reader = message.get_root_as_reader()?;
                result.imbue(cap_table);
                Ok(result)
            }
        }
    }
}

enum ClientVariant<VatId>
where
    VatId: 'static,
{
    Import(Rc<RefCell<ImportClient<VatId>>>),
    Pipeline(Rc<RefCell<PipelineClient<VatId>>>),
    Promise(Rc<RefCell<PromiseClient<VatId>>>),
}

struct Client<VatId>
where
    VatId: 'static,
{
    connection_state: Rc<ConnectionState<VatId>>,
    variant: ClientVariant<VatId>,
    flow_controller: Rc<RefCell<Option<Box<dyn crate::FlowController>>>>,
}

enum WeakClientVariant<VatId>
where
    VatId: 'static,
{
    Import(Weak<RefCell<ImportClient<VatId>>>),
    Pipeline(Weak<RefCell<PipelineClient<VatId>>>),
    Promise(Weak<RefCell<PromiseClient<VatId>>>),
}

struct WeakClient<VatId>
where
    VatId: 'static,
{
    connection_state: Weak<ConnectionState<VatId>>,
    variant: WeakClientVariant<VatId>,
    flow_controller: Weak<RefCell<Option<Box<dyn crate::FlowController>>>>,
}

impl<VatId> WeakClient<VatId>
where
    VatId: 'static,
{
    fn upgrade(&self) -> Option<Client<VatId>> {
        let variant = match &self.variant {
            WeakClientVariant::Import(ic) => ClientVariant::Import(ic.upgrade()?),
            WeakClientVariant::Pipeline(pc) => ClientVariant::Pipeline(pc.upgrade()?),
            WeakClientVariant::Promise(pc) => ClientVariant::Promise(pc.upgrade()?),
        };
        let connection_state = self.connection_state.upgrade()?;
        let flow_controller = self.flow_controller.upgrade()?;
        Some(Client {
            connection_state,
            variant,
            flow_controller,
        })
    }
}

struct ImportClient<VatId>
where
    VatId: 'static,
{
    connection_state: Rc<ConnectionState<VatId>>,
    import_id: ImportId,

    /// Number of times we've received this import from the peer.
    remote_ref_count: u32,
}

impl<VatId> Drop for ImportClient<VatId> {
    fn drop(&mut self) {
        let connection_state = self.connection_state.clone();

        assert!(connection_state
            .client_downcast_map
            .borrow_mut()
            .remove(&((self) as *const _ as usize))
            .is_some());

        // Remove the corresponding entry of the imports table.
        // Note: the C++ implementation checks here pointer equality between self and
        // the entry in the imports table, but as far as I can tell the check should
        // always pass because of how we construct ImportClient in import().
        connection_state
            .imports
            .borrow_mut()
            .slots
            .remove(&self.import_id);

        // Send a message releasing our remote references.
        let mut tmp = connection_state.connection.borrow_mut();
        if let (true, Ok(c)) = (self.remote_ref_count > 0, tmp.as_mut()) {
            let mut message = c.new_outgoing_message(10);
            {
                let root: message::Builder = message.get_body().unwrap().init_as();
                let mut release = root.init_release();
                release.set_id(self.import_id);
                release.set_reference_count(self.remote_ref_count);
            }
            let _ = message.send();
        }
    }
}

impl<VatId> ImportClient<VatId>
where
    VatId: 'static,
{
    fn new(
        connection_state: &Rc<ConnectionState<VatId>>,
        import_id: ImportId,
    ) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            connection_state: connection_state.clone(),
            import_id,
            remote_ref_count: 0,
        }))
    }

    fn add_remote_ref(&mut self) {
        self.remote_ref_count += 1;
    }
}

impl<VatId> From<Rc<RefCell<ImportClient<VatId>>>> for Client<VatId> {
    fn from(client: Rc<RefCell<ImportClient<VatId>>>) -> Self {
        let connection_state = client.borrow().connection_state.clone();
        Self::new(&connection_state, ClientVariant::Import(client))
    }
}

/// A `ClientHook` representing a pipelined promise.  Always wrapped in `PromiseClient`.
struct PipelineClient<VatId>
where
    VatId: 'static,
{
    connection_state: Rc<ConnectionState<VatId>>,
    question_ref: Rc<RefCell<QuestionRef<VatId>>>,
    ops: Vec<PipelineOp>,
}

impl<VatId> PipelineClient<VatId>
where
    VatId: 'static,
{
    fn new(
        connection_state: &Rc<ConnectionState<VatId>>,
        question_ref: Rc<RefCell<QuestionRef<VatId>>>,
        ops: Vec<PipelineOp>,
    ) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            connection_state: connection_state.clone(),
            question_ref,
            ops,
        }))
    }
}

impl<VatId> From<Rc<RefCell<PipelineClient<VatId>>>> for Client<VatId> {
    fn from(client: Rc<RefCell<PipelineClient<VatId>>>) -> Self {
        let connection_state = client.borrow().connection_state.clone();
        Self::new(&connection_state, ClientVariant::Pipeline(client))
    }
}

impl<VatId> Drop for PipelineClient<VatId> {
    fn drop(&mut self) {
        assert!(self
            .connection_state
            .client_downcast_map
            .borrow_mut()
            .remove(&((self) as *const _ as usize))
            .is_some());
    }
}

/// A `ClientHook` that initially wraps one client and then, later on, redirects
/// to some other client.
struct PromiseClient<VatId>
where
    VatId: 'static,
{
    connection_state: Rc<ConnectionState<VatId>>,
    is_resolved: bool,
    cap: Box<dyn ClientHook>,
    import_id: Option<ImportId>,
    received_call: bool,
    resolution_waiters: crate::sender_queue::SenderQueue<(), Box<dyn ClientHook>>,
}

impl<VatId> PromiseClient<VatId> {
    fn new(
        connection_state: &Rc<ConnectionState<VatId>>,
        initial: Box<dyn ClientHook>,
        import_id: Option<ImportId>,
    ) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            connection_state: connection_state.clone(),
            is_resolved: false,
            cap: initial,
            import_id,
            received_call: false,
            resolution_waiters: crate::sender_queue::SenderQueue::new(),
        }))
    }

    fn resolve(&mut self, replacement: Result<Box<dyn ClientHook>, Error>) {
        let (mut replacement, is_error) = match replacement {
            Ok(v) => (v, false),
            Err(e) => (broken::new_cap(e), true),
        };
        let connection_state = self.connection_state.clone();
        let is_connected = connection_state.connection.borrow().is_ok();
        let replacement_brand = replacement.get_brand();
        if replacement_brand != connection_state.get_brand()
            && self.received_call
            && !is_error
            && is_connected
        {
            // The new capability is hosted locally, not on the remote machine.  And, we had made calls
            // to the promise.  We need to make sure those calls echo back to us before we allow new
            // calls to go directly to the local capability, so we need to set a local embargo and send
            // a `Disembargo` to echo through the peer.
            let (fulfiller, promise) = oneshot::channel::<Result<(), Error>>();
            let promise = promise
                .map_err(crate::canceled_to_error)
                .and_then(future::ready);
            let embargo = Embargo::new(fulfiller);
            let embargo_id = connection_state.embargoes.borrow_mut().push(embargo);

            let mut message = connection_state
                .new_outgoing_message(50)
                .expect("no connection?"); // XXX size hint
            {
                let root: message::Builder = message.get_body().unwrap().init_as();
                let mut disembargo = root.init_disembargo();
                disembargo
                    .reborrow()
                    .init_context()
                    .set_sender_loopback(embargo_id);
                let target = disembargo.init_target();

                let redirect = connection_state.write_target(&*self.cap, target);
                if redirect.is_some() {
                    panic!("Original promise target should always be from this RPC connection.")
                }
            }

            // Make a promise which resolves to `replacement` as soon as the `Disembargo` comes back.
            let embargo_promise = promise.map_ok(move |()| replacement);

            let mut queued_client = queued::Client::new(None);
            let weak_queued = Rc::downgrade(&queued_client.inner);

            queued_client.drive(embargo_promise.then(move |r| {
                if let Some(q) = weak_queued.upgrade() {
                    queued::ClientInner::resolve(&q, r);
                }
                Promise::ok(())
            }));

            // We need to queue up calls in the meantime, so we'll resolve ourselves to a local promise
            // client instead.
            replacement = Box::new(queued_client);

            let _ = message.send();
        }

        for ((), waiter) in self.resolution_waiters.drain() {
            let _ = waiter.send(replacement.clone());
        }

        let old_cap = mem::replace(&mut self.cap, replacement);
        connection_state.add_task(async move {
            drop(old_cap);
            Ok(())
        });

        self.is_resolved = true;
    }
}

impl<VatId> Drop for PromiseClient<VatId> {
    fn drop(&mut self) {
        let self_ptr = (self) as *const _ as usize;

        if let Some(id) = self.import_id {
            // This object is representing an import promise.  That means the import table may still
            // contain a pointer back to it.  Remove that pointer.  Note that we have to verify that
            // the import still exists and the pointer still points back to this object because this
            // object may actually outlive the import.
            let slots = &mut self.connection_state.imports.borrow_mut().slots;
            if let Some(import) = slots.get_mut(&id) {
                if let Some(c) = &import.app_client {
                    if let Some(cs) = c.upgrade() {
                        if cs.get_ptr() == self_ptr {
                            import.app_client = None;
                        }
                    }
                }
            }
        }

        assert!(self
            .connection_state
            .client_downcast_map
            .borrow_mut()
            .remove(&self_ptr)
            .is_some());
    }
}

impl<VatId> From<Rc<RefCell<PromiseClient<VatId>>>> for Client<VatId> {
    fn from(client: Rc<RefCell<PromiseClient<VatId>>>) -> Self {
        let connection_state = client.borrow().connection_state.clone();
        Self::new(&connection_state, ClientVariant::Promise(client))
    }
}

impl<VatId> Client<VatId> {
    fn new(connection_state: &Rc<ConnectionState<VatId>>, variant: ClientVariant<VatId>) -> Self {
        let client = Self {
            connection_state: connection_state.clone(),
            variant,
            flow_controller: Rc::new(RefCell::new(None)),
        };
        let weak = client.downgrade();

        // XXX arguably, this should go in each of the variant's constructors.
        connection_state
            .client_downcast_map
            .borrow_mut()
            .insert(client.get_ptr(), weak);
        client
    }
    fn downgrade(&self) -> WeakClient<VatId> {
        let variant = match &self.variant {
            ClientVariant::Import(import_client) => {
                WeakClientVariant::Import(Rc::downgrade(import_client))
            }
            ClientVariant::Pipeline(pipeline_client) => {
                WeakClientVariant::Pipeline(Rc::downgrade(pipeline_client))
            }
            ClientVariant::Promise(promise_client) => {
                WeakClientVariant::Promise(Rc::downgrade(promise_client))
            }
        };
        WeakClient {
            connection_state: Rc::downgrade(&self.connection_state),
            variant,
            flow_controller: Rc::downgrade(&self.flow_controller),
        }
    }

    fn from_ptr(ptr: usize, connection_state: &ConnectionState<VatId>) -> Option<Self> {
        match connection_state.client_downcast_map.borrow().get(&ptr) {
            Some(c) => c.upgrade(),
            None => None,
        }
    }

    fn write_target(
        &self,
        mut target: crate::rpc_capnp::message_target::Builder,
    ) -> Option<Box<dyn ClientHook>> {
        match &self.variant {
            ClientVariant::Import(import_client) => {
                target.set_imported_cap(import_client.borrow().import_id);
                None
            }
            ClientVariant::Pipeline(pipeline_client) => {
                let mut builder = target.init_promised_answer();
                let question_ref = &pipeline_client.borrow().question_ref;
                builder.set_question_id(question_ref.borrow().id);
                let mut transform =
                    builder.init_transform(pipeline_client.borrow().ops.len() as u32);
                for idx in 0..pipeline_client.borrow().ops.len() {
                    if let ::capnp::private::capability::PipelineOp::GetPointerField(ordinal) =
                        pipeline_client.borrow().ops[idx]
                    {
                        transform
                            .reborrow()
                            .get(idx as u32)
                            .set_get_pointer_field(ordinal);
                    }
                }
                None
            }
            ClientVariant::Promise(promise_client) => {
                promise_client.borrow_mut().received_call = true;
                self.connection_state
                    .write_target(&*promise_client.borrow().cap, target)
            }
        }
    }

    fn write_descriptor(
        &self,
        mut descriptor: cap_descriptor::Builder,
    ) -> capnp::Result<Option<u32>> {
        match &self.variant {
            ClientVariant::Import(import_client) => {
                descriptor.set_receiver_hosted(import_client.borrow().import_id);
                Ok(None)
            }
            ClientVariant::Pipeline(pipeline_client) => {
                let mut promised_answer = descriptor.init_receiver_answer();
                let question_ref = &pipeline_client.borrow().question_ref;
                promised_answer.set_question_id(question_ref.borrow().id);
                let mut transform =
                    promised_answer.init_transform(pipeline_client.borrow().ops.len() as u32);
                for idx in 0..pipeline_client.borrow().ops.len() {
                    if let ::capnp::private::capability::PipelineOp::GetPointerField(ordinal) =
                        pipeline_client.borrow().ops[idx]
                    {
                        transform
                            .reborrow()
                            .get(idx as u32)
                            .set_get_pointer_field(ordinal);
                    }
                }

                Ok(None)
            }
            ClientVariant::Promise(promise_client) => {
                promise_client.borrow_mut().received_call = true;

                let cap = promise_client.borrow().cap.clone();
                ConnectionState::write_descriptor(&self.connection_state, cap, descriptor)
            }
        }
    }
}

impl<VatId> Clone for Client<VatId> {
    fn clone(&self) -> Self {
        let variant = match &self.variant {
            ClientVariant::Import(import_client) => ClientVariant::Import(import_client.clone()),
            ClientVariant::Pipeline(pipeline_client) => {
                ClientVariant::Pipeline(pipeline_client.clone())
            }
            ClientVariant::Promise(promise_client) => {
                ClientVariant::Promise(promise_client.clone())
            }
        };
        Self {
            connection_state: self.connection_state.clone(),
            variant,
            flow_controller: self.flow_controller.clone(),
        }
    }
}

impl<VatId> ClientHook for Client<VatId> {
    fn add_ref(&self) -> Box<dyn ClientHook> {
        Box::new(self.clone())
    }
    fn new_call(
        &self,
        interface_id: u64,
        method_id: u16,
        size_hint: Option<::capnp::MessageSize>,
    ) -> ::capnp::capability::Request<any_pointer::Owned, any_pointer::Owned> {
        let request: Box<dyn RequestHook> =
            match Request::new(self.connection_state.clone(), size_hint, self.clone()) {
                Ok(mut request) => {
                    {
                        let mut call_builder = request.init_call();
                        call_builder.set_interface_id(interface_id);
                        call_builder.set_method_id(method_id);
                    }
                    Box::new(request)
                }
                Err(e) => Box::new(broken::Request::new(e, None)),
            };

        ::capnp::capability::Request::new(request)
    }

    fn call(
        &self,
        interface_id: u64,
        method_id: u16,
        params: Box<dyn ParamsHook>,
        mut results: Box<dyn ResultsHook>,
    ) -> Promise<(), Error> {
        // Implement call() by copying params and results messages.

        let maybe_request = params.get().and_then(|p| {
            let mut request = p
                .target_size()
                .map(|s| self.new_call(interface_id, method_id, Some(s)))?;
            request.get().set_as(p)?;
            Ok(request)
        });

        match maybe_request {
            Err(e) => Promise::err(e),
            Ok(request) => {
                let ::capnp::capability::RemotePromise { promise, .. } = request.send();

                Promise::from_future(async move {
                    let response = promise.await?;
                    results.get()?.set_as(response.get()?)?;
                    Ok(())
                })
            }
        }
        // TODO implement this in terms of direct tail call.
        // We can and should propagate cancellation.
        // (TODO ?)
        // context -> allowCancellation();

        //results.direct_tail_call(request.hook)
    }

    fn get_ptr(&self) -> usize {
        match &self.variant {
            ClientVariant::Import(import_client) => (&*import_client.borrow()) as *const _ as usize,
            ClientVariant::Pipeline(pipeline_client) => {
                (&*pipeline_client.borrow()) as *const _ as usize
            }
            ClientVariant::Promise(promise_client) => {
                (&*promise_client.borrow()) as *const _ as usize
            }
        }
    }

    fn get_brand(&self) -> usize {
        self.connection_state.get_brand()
    }

    fn get_resolved(&self) -> Option<Box<dyn ClientHook>> {
        match &self.variant {
            ClientVariant::Import(_import_client) => None,
            ClientVariant::Pipeline(_pipeline_client) => None,
            ClientVariant::Promise(promise_client) => {
                if promise_client.borrow().is_resolved {
                    Some(promise_client.borrow().cap.clone())
                } else {
                    None
                }
            }
        }
    }

    fn when_more_resolved(&self) -> Option<Promise<Box<dyn ClientHook>, Error>> {
        match &self.variant {
            ClientVariant::Import(_import_client) => None,
            ClientVariant::Pipeline(_pipeline_client) => None,
            ClientVariant::Promise(promise_client) => {
                let mut client = promise_client.borrow_mut();
                if client.is_resolved {
                    Some(Promise::ok(client.cap.clone()))
                } else {
                    Some(client.resolution_waiters.push(()))
                }
            }
        }
    }

    fn when_resolved(&self) -> Promise<(), Error> {
        default_when_resolved_impl(self)
    }
}

pub(crate) fn default_when_resolved_impl<C>(client: &C) -> Promise<(), Error>
where
    C: ClientHook,
{
    match client.when_more_resolved() {
        Some(promise) => {
            Promise::from_future(promise.and_then(|resolution| resolution.when_resolved()))
        }
        None => Promise::ok(()),
    }
}

// ===================================

struct SingleCapPipeline {
    cap: Box<dyn ClientHook>,
}

impl SingleCapPipeline {
    fn new(cap: Box<dyn ClientHook>) -> Self {
        Self { cap }
    }
}

impl PipelineHook for SingleCapPipeline {
    fn add_ref(&self) -> Box<dyn PipelineHook> {
        Box::new(Self {
            cap: self.cap.clone(),
        })
    }
    fn get_pipelined_cap(&self, ops: &[PipelineOp]) -> Box<dyn ClientHook> {
        if ops.is_empty() {
            self.cap.add_ref()
        } else {
            broken::new_cap(Error::failed("Invalid pipeline transform.".to_string()))
        }
    }
}

#[cfg(test)]
mod terminalization_tests {
    use super::*;
    use crate::VatNetwork;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    type Side = crate::rpc_twoparty_capnp::Side;

    fn connection() -> (
        TaskSet<Error>,
        Rc<ConnectionState<Side>>,
        oneshot::Receiver<Promise<(), Error>>,
    ) {
        let mut network = crate::twoparty::VatNetwork::new(
            futures::io::Cursor::new(Vec::<u8>::new()),
            futures::io::Cursor::new(Vec::<u8>::new()),
            Side::Client,
            Default::default(),
        );
        let (tx, rx) = oneshot::channel();
        let (tasks, state) = ConnectionState::new(
            broken::new_cap(Error::failed("unused bootstrap".into())),
            network.connect(Side::Server).unwrap(),
            tx,
        );
        (tasks, state, rx)
    }

    fn waiting_pipeline(state: &Rc<ConnectionState<Side>>) -> Pipeline<Side> {
        let id = state.questions.borrow_mut().push(Question::new());
        let (tx, rx) = oneshot::channel();
        let question = Rc::new(RefCell::new(QuestionRef::new(state.clone(), id, tx)));
        state.questions.borrow_mut().find(id).unwrap().self_ref = Some(Rc::downgrade(&question));
        Pipeline::new(
            state,
            question,
            Some(Promise::from_future(
                rx.map_err(crate::canceled_to_error).and_then(|x| x),
            )),
        )
    }

    struct DropAction(Box<dyn Fn()>);
    impl Drop for DropAction {
        fn drop(&mut self) {
            (self.0)();
        }
    }
    #[derive(Clone)]
    struct DropServer(Rc<DropAction>);
    impl capnp::capability::Server for DropServer {
        fn dispatch_call(
            self,
            _: u64,
            _: u16,
            _: capnp::capability::Params<any_pointer::Owned>,
            _: capnp::capability::Results<any_pointer::Owned>,
        ) -> capnp::capability::DispatchCallResult {
            capnp::capability::DispatchCallResult {
                promise: Promise::ok(()),
                is_streaming: false,
            }
        }
        fn as_ptr(&self) -> usize {
            Rc::as_ptr(&self.0) as usize
        }
    }
    fn export(state: &Rc<ConnectionState<Side>>, drop: impl Fn() + 'static) {
        let hook = Box::new(local::Client::new(DropServer(Rc::new(DropAction(
            Box::new(drop),
        )))));
        let ptr = hook.get_ptr();
        let id = state.exports.borrow_mut().push(Export::new(hook));
        state.exports_by_cap.borrow_mut().insert(ptr, id);
    }

    #[test]
    fn disconnect_rejects_waiting_pipeline_without_polling_driver() {
        let (_tasks, state, completion) = connection();
        let pipeline = waiting_pipeline(&state);
        let waiter = pipeline.when_resolved();
        let promised = pipeline.get_pipelined_cap(&[]);
        let resolution = promised.when_resolved();
        let imported = ConnectionState::import(&state, 7, true);
        let import_resolution = imported.when_resolved();
        let (embargo_tx, embargo_rx) = oneshot::channel();
        state.embargoes.borrow_mut().push(Embargo::new(embargo_tx));
        state.disconnect(Error::disconnected("terminal cause".into()));
        assert!(matches!(waiter.now_or_never(), Some(Ok(()))));
        assert!(resolution.now_or_never().is_some());
        assert!(promised.when_resolved().now_or_never().is_some());
        assert!(import_resolution.now_or_never().is_some());
        assert!(imported.when_resolved().now_or_never().is_some());
        assert!(matches!(
            imported.new_call(0, 0, None).send().promise.now_or_never(),
            Some(Err(_))
        ));
        assert!(matches!(embargo_rx.now_or_never(), Some(Ok(Err(_)))));
        assert!(matches!(
            promised.new_call(0, 0, None).send().promise.now_or_never(),
            Some(Err(_))
        ));
        assert!(
            matches!(&pipeline.state.borrow().variant, PipelineVariant::Broken(e) if e.extra == "terminal cause")
        );
        // A scheduled resolver completing later cannot overwrite terminal cause.
        PipelineState::resolve(&pipeline.state, Err(Error::failed("late resolver".into())));
        assert!(
            matches!(&pipeline.state.borrow().variant, PipelineVariant::Broken(e) if e.extra == "terminal cause")
        );
        assert!(matches!(completion.now_or_never(), Some(Ok(_))));
    }

    #[test]
    fn disconnect_is_terminal_before_reentrant_export_drop() {
        let (_tasks, state, completion) = connection();
        let weak = Rc::downgrade(&state);
        let ran = Rc::new(Cell::new(0));
        let observed = ran.clone();
        export(&state, move || {
            observed.set(observed.get() + 1);
            let state = weak.upgrade().unwrap();
            assert!(state.new_outgoing_message(0).is_err());
            assert_eq!(state.exports.borrow().iter().count(), 0);
            assert!(state.exports_by_cap.borrow().is_empty());
            state.disconnect(Error::disconnected("reentrant".into()));
        });
        state.disconnect(Error::disconnected("terminal cause".into()));
        assert_eq!(ran.get(), 1);
        assert!(matches!(completion.now_or_never(), Some(Ok(_))));
        state.disconnect(Error::disconnected("repeated".into()));
        assert_eq!(ran.get(), 1);
    }

    #[test]
    fn destructor_panic_does_not_skip_exports_or_disconnect_completion() {
        let (_tasks, state, completion) = connection();
        export(&state, || panic!("first destructor"));
        let drops = Rc::new(Cell::new(0));
        let observed = drops.clone();
        export(&state, move || observed.set(observed.get() + 1));
        let panic = catch_unwind(AssertUnwindSafe(|| {
            state.disconnect(Error::disconnected("terminal cause".into()))
        }))
        .unwrap_err();
        assert_eq!(panic.downcast_ref::<&str>(), Some(&"first destructor"));
        assert_eq!(drops.get(), 1);
        assert!(state.connection.borrow().is_err());
        assert!(state.disconnect_fulfiller.borrow().is_none());
        assert!(matches!(completion.now_or_never(), Some(Ok(_))));
        state.disconnect(Error::disconnected("repeated".into()));
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn multiple_destructor_panics_preserve_first_cause() {
        // The unpatched implementation aborts on a second destructor panic.
        // Isolate it so the regression reports failure without killing the suite.
        const CHILD: &str = "CAPNP_TERMINALIZATION_PANIC_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "rpc::terminalization_tests::multiple_destructor_panics_preserve_first_cause",
                    "--nocapture",
                ])
                .env(CHILD, "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        struct PanickingPayload;
        impl Drop for PanickingPayload {
            fn drop(&mut self) {
                panic!("secondary payload destructor");
            }
        }
        let (_tasks, state, completion) = connection();
        export(&state, || panic!("first destructor"));
        export(&state, || std::panic::panic_any(PanickingPayload));
        let drops = Rc::new(Cell::new(0));
        let observed = drops.clone();
        export(&state, move || observed.set(observed.get() + 1));
        // Export-owned cancellation handles are independent drop targets. These
        // synthetic futures test those handles, not the TaskSet-owned resolver.
        let resolve_drops = Rc::new(Cell::new(0));
        for i in 0..2 {
            let observed = resolve_drops.clone();
            let on_drop = DropAction(Box::new(move || {
                observed.set(observed.get() + 1);
                assert_ne!(i, 0, "resolve destructor");
            }));
            let mut export = Export::new(broken::new_cap(Error::failed("unused".into())));
            export.resolve_op = Promise::from_future(async move {
                let _on_drop = on_drop;
                future::pending::<Result<(), Error>>().await
            });
            state.exports.borrow_mut().push(export);
        }
        let shared = Rc::new(RefCell::new(Some(state.clone())));
        let mut disconnector = Disconnector::new(shared.clone());
        let panic =
            catch_unwind(AssertUnwindSafe(|| (&mut disconnector).now_or_never())).unwrap_err();
        assert_eq!(panic.downcast_ref::<&str>(), Some(&"first destructor"));
        assert_eq!(drops.get(), 1);
        assert_eq!(resolve_drops.get(), 2);
        assert!(state.connection.borrow().is_err());
        assert!(state.disconnect_fulfiller.borrow().is_none());
        assert!(matches!(completion.now_or_never(), Some(Ok(_))));
        // Repeated initiation and later completion neither panic nor drop twice.
        state.disconnect(Error::disconnected("repeated".into()));
        let mut shutdown = state.disconnect_promise.borrow().clone().unwrap();
        assert!(matches!((&mut disconnector).now_or_never(), Some(Ok(()))));
        assert!(matches!((&mut shutdown).now_or_never(), Some(Ok(()))));
        assert!(matches!((&mut disconnector).now_or_never(), Some(Ok(()))));
        assert_eq!(drops.get(), 1);
        assert_eq!(resolve_drops.get(), 2);
    }

    struct PendingRead;

    impl futures::io::AsyncRead for PendingRead {
        fn poll_read(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            _: &mut [u8],
        ) -> Poll<std::io::Result<usize>> {
            Poll::Pending
        }
    }

    // Keep real two-party connections, but control the network driver's result
    // independently of connection shutdown to exercise a retained failed owner.
    struct ControlledNetwork {
        inner: crate::twoparty::VatNetwork<PendingRead>,
        driver_result: Option<oneshot::Receiver<Result<(), Error>>>,
    }

    impl VatNetwork<Side> for ControlledNetwork {
        fn connect(&mut self, side: Side) -> Option<Box<dyn crate::Connection<Side>>> {
            self.inner.connect(side)
        }

        fn accept(&mut self) -> Promise<Box<dyn crate::Connection<Side>>, Error> {
            self.inner.accept()
        }

        fn drive_until_shutdown(&mut self) -> Promise<(), Error> {
            let result = self.driver_result.take().unwrap();
            Promise::from_future(async move { result.await.unwrap() })
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum ResolverOwnerState {
        NeverPolled,
        Pending,
        Failed,
    }

    fn assert_rpc_drop_releases_export_resolution(owner_state: ResolverOwnerState) {
        let (driver_tx, driver_rx) = oneshot::channel();
        let network = ControlledNetwork {
            inner: crate::twoparty::VatNetwork::new(
                PendingRead,
                futures::io::Cursor::new(Vec::<u8>::new()),
                Side::Client,
                Default::default(),
            ),
            driver_result: Some(driver_rx),
        };
        let mut rpc = crate::RpcSystem::new(Box::new(network), None);
        let bootstrap: capnp::capability::Client = rpc.bootstrap(Side::Server);
        let state = rpc.connection_state.borrow().as_ref().unwrap().clone();
        let import = ConnectionState::import(&state, 42, false);
        let drops = Rc::new(Cell::new(0));
        let observed = drops.clone();
        let on_drop = DropAction(Box::new(move || observed.set(observed.get() + 1)));
        let polls = Rc::new(Cell::new(0));
        let observed_polls = polls.clone();
        let promise = Promise::from_future(async move {
            let _on_drop = on_drop;
            future::poll_fn(move |_| {
                observed_polls.set(observed_polls.get() + 1);
                Poll::<Result<Box<dyn ClientHook>, Error>>::Pending
            })
            .await
        });
        let id = state
            .exports
            .borrow_mut()
            .push(Export::new(broken::new_cap(Error::failed("unused".into()))));
        let resolve_op = ConnectionState::resolve_exported_promise(&state, id, promise);
        state.exports.borrow_mut().find(id).unwrap().resolve_op = resolve_op;

        if !matches!(owner_state, ResolverOwnerState::NeverPolled) {
            assert!((&mut rpc).now_or_never().is_none());
            assert!(polls.get() > 0, "resolver must have returned Pending");
        } else {
            assert_eq!(polls.get(), 0);
        }
        if matches!(owner_state, ResolverOwnerState::Failed) {
            driver_tx
                .send(Err(Error::failed("controlled driver failure".into())))
                .unwrap();
            let result = (0..32)
                .find_map(|_| (&mut rpc).now_or_never())
                .expect("driver must finish after its explicit failure signal");
            assert_eq!(result.unwrap_err().extra, "controlled driver failure");
            assert_eq!(drops.get(), 0, "a retained failed owner may retain tasks");
        }

        state.disconnect(Error::disconnected("terminal cause".into()));
        // Disconnect signals cancellation; destruction of the owner releases
        // the queued or suspended resolver without polling that owner again.
        let polls_before_drop = polls.get();
        assert_eq!(drops.get(), 0);
        drop(rpc);
        assert_eq!(drops.get(), 1, "owner state: {owner_state:?}");
        assert_eq!(polls.get(), polls_before_drop);
        // Keep external handles alive until after the release assertion.
        drop((bootstrap, import, state));
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn rpc_drop_releases_never_polled_export_resolution() {
        assert_rpc_drop_releases_export_resolution(ResolverOwnerState::NeverPolled);
    }

    #[test]
    fn rpc_drop_releases_pending_export_resolution() {
        assert_rpc_drop_releases_export_resolution(ResolverOwnerState::Pending);
    }

    #[test]
    fn rpc_drop_releases_export_resolution_after_driver_failure() {
        assert_rpc_drop_releases_export_resolution(ResolverOwnerState::Failed);
    }

    fn preconstructed_request_during_export_drop(streaming: bool) {
        let (_tasks, state, completion) = connection();
        let target = ConnectionState::import(&state, 17, false);
        let mut request = target.new_call(0, 0, None);
        let drops = Rc::new(Cell::new(0));
        let observed = drops.clone();
        let parameter = Box::new(local::Client::new(DropServer(Rc::new(DropAction(
            Box::new(move || observed.set(observed.get() + 1)),
        )))));
        request.get().set_as_capability(parameter);
        let request = RefCell::new(Some(request));
        let outcome = Rc::new(RefCell::new(None));
        let result = outcome.clone();
        // The request already owns an outgoing transport message. Sending it
        // from a destructor must not repopulate the drained export/question tables.
        export(&state, move || {
            let request = request.borrow_mut().take().unwrap();
            *result.borrow_mut() = if streaming {
                request.hook.send_streaming().now_or_never()
            } else {
                let pending = request.send();
                let result = pending.promise.now_or_never().map(|r| r.map(|_| ()));
                drop(pending.pipeline);
                result
            };
        });
        assert_eq!(state.questions.borrow().iter().count(), 0);
        state.disconnect(Error::disconnected("terminal cause".into()));
        assert_eq!(
            state.exports.borrow().iter().count(),
            0,
            "preconstructed request created an owning export after disconnect"
        );
        assert!(state.exports_by_cap.borrow().is_empty());
        assert_eq!(
            state.questions.borrow().iter().count(),
            0,
            "preconstructed request created a question after disconnect"
        );
        assert_eq!(
            drops.get(),
            1,
            "request parameter ownership must be released"
        );
        let error = outcome
            .borrow_mut()
            .take()
            .expect("terminal send must settle immediately")
            .unwrap_err();
        assert_eq!(error.kind, capnp::ErrorKind::Disconnected);
        assert_eq!(error.extra, "terminal cause");
        assert!(matches!(completion.now_or_never(), Some(Ok(_))));
        state.disconnect(Error::disconnected("repeated".into()));
        assert_eq!(state.exports.borrow().iter().count(), 0);
        assert_eq!(state.questions.borrow().iter().count(), 0);
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn preconstructed_ordinary_request_is_rejected_during_export_drop() {
        preconstructed_request_during_export_drop(false);
    }

    #[test]
    fn preconstructed_streaming_request_is_rejected_during_export_drop() {
        preconstructed_request_during_export_drop(true);
    }
    // Exercise public capability hooks at both sides of descriptor admission.
    #[derive(Clone, Copy, PartialEq)]
    enum PreparationCallback {
        Identity,
        Resolution,
        Drop,
    }

    struct PreparationHook {
        inner: Box<dyn ClientHook>,
        callback: Rc<RefCell<Option<Box<dyn FnOnce()>>>>,
        stage: PreparationCallback,
        is_copy: bool,
    }

    impl PreparationHook {
        fn fire(&self, stage: PreparationCallback) {
            if self.stage == stage {
                let callback = self.callback.borrow_mut().take();
                if let Some(callback) = callback {
                    callback();
                }
            }
        }
    }

    impl Drop for PreparationHook {
        fn drop(&mut self) {
            if self.is_copy {
                self.fire(PreparationCallback::Drop);
            }
        }
    }

    impl ClientHook for PreparationHook {
        fn add_ref(&self) -> Box<dyn ClientHook> {
            Box::new(Self {
                inner: self.inner.clone(),
                callback: self.callback.clone(),
                stage: self.stage,
                is_copy: true,
            })
        }
        fn new_call(
            &self,
            interface: u64,
            method: u16,
            hint: Option<capnp::MessageSize>,
        ) -> capnp::capability::Request<any_pointer::Owned, any_pointer::Owned> {
            self.inner.new_call(interface, method, hint)
        }
        fn call(
            &self,
            interface: u64,
            method: u16,
            params: Box<dyn ParamsHook>,
            results: Box<dyn ResultsHook>,
        ) -> Promise<(), Error> {
            self.inner.call(interface, method, params, results)
        }
        fn get_brand(&self) -> usize {
            self.inner.get_brand()
        }
        fn get_ptr(&self) -> usize {
            self.fire(PreparationCallback::Identity);
            self.inner.get_ptr()
        }
        fn get_resolved(&self) -> Option<Box<dyn ClientHook>> {
            None
        }
        fn when_more_resolved(&self) -> Option<Promise<Box<dyn ClientHook>, Error>> {
            self.fire(PreparationCallback::Resolution);
            self.inner.when_more_resolved()
        }
        fn when_resolved(&self) -> Promise<(), Error> {
            default_when_resolved_impl(self)
        }
    }

    fn preparation_hook(
        state: &Rc<ConnectionState<Side>>,
        stage: PreparationCallback,
    ) -> Box<dyn ClientHook> {
        let weak = Rc::downgrade(state);
        // A reflected import takes the no-export path; dropping its temporary
        // descriptor hook can disconnect after the export checks have finished.
        let inner = if stage == PreparationCallback::Drop {
            ConnectionState::import(state, 23, false)
        } else {
            Box::new(local::Client::new(DropServer(Rc::new(DropAction(
                Box::new(|| {}),
            )))))
        };
        Box::new(PreparationHook {
            inner,
            callback: Rc::new(RefCell::new(Some(Box::new(move || {
                weak.upgrade()
                    .unwrap()
                    .disconnect(Error::disconnected("preparation stopped".into()));
            })))),
            stage,
            is_copy: false,
        })
    }

    fn assert_empty_bookkeeping(state: &ConnectionState<Side>) {
        assert_eq!(
            state.exports.borrow().iter().count(),
            0,
            "exports survived rejected preparation"
        );
        assert!(state.exports_by_cap.borrow().is_empty());
        assert_eq!(
            state.questions.borrow().iter().count(),
            0,
            "question admitted after terminalization"
        );
    }

    fn assert_preparation_error(error: Error) {
        assert_eq!(error.kind, capnp::ErrorKind::Disconnected);
        assert_eq!(error.extra, "preparation stopped");
    }

    fn request_preparation_disconnect(stage: PreparationCallback, streaming: bool, tail: bool) {
        let (_tasks, state, _completion) = connection();
        let target = ConnectionState::import(&state, 17, false);
        // Retain an already-created streaming controller through the transition.
        if streaming {
            let client = Client::from_ptr(target.get_ptr(), &state).unwrap();
            let (flow, task) = state.connection.borrow_mut().as_mut().unwrap().new_stream();
            *client.flow_controller.borrow_mut() = Some(flow);
            state.add_task(task);
        }
        let mut request = target.new_call(0, 0, None);
        let mut params = request
            .get()
            .initn_as::<capnp::any_pointer_list::Builder>(2);
        // This first descriptor records ownership before the second callback
        // disconnects. Terminal rejection must leave neither ownership edge.
        params
            .reborrow()
            .get(0)
            .set_as_capability(Box::new(local::Client::new(DropServer(Rc::new(
                DropAction(Box::new(|| {})),
            )))));
        params
            .get(1)
            .set_as_capability(preparation_hook(&state, stage));
        assert!(state.connection.borrow().is_ok());
        if tail {
            assert!(request.hook.tail_send().is_none());
        } else if streaming {
            let result = request.hook.send_streaming();
            assert_empty_bookkeeping(&state);
            assert_preparation_error(
                result
                    .now_or_never()
                    .expect("terminal streaming result")
                    .unwrap_err(),
            );
        } else {
            let result = request.send();
            assert_empty_bookkeeping(&state);
            assert_preparation_error(
                result
                    .promise
                    .now_or_never()
                    .expect("terminal response")
                    .err()
                    .unwrap(),
            );
            let cap = result.pipeline.as_cap();
            assert_preparation_error(
                cap.new_call(0, 0, None)
                    .send()
                    .promise
                    .now_or_never()
                    .expect("terminal pipeline")
                    .err()
                    .unwrap(),
            );
        }
        assert!(state.connection.borrow().is_err());
        assert_empty_bookkeeping(&state);
        state.disconnect(Error::failed("later cause".into()));
        assert_empty_bookkeeping(&state);
        assert_preparation_error(state.connection.borrow().as_ref().err().unwrap().clone());
    }

    #[test]
    fn ordinary_preparation_rejects_exports_after_callback() {
        request_preparation_disconnect(PreparationCallback::Identity, false, false);
    }

    #[test]
    fn streaming_preparation_rejects_exports_after_callback() {
        request_preparation_disconnect(PreparationCallback::Identity, true, false);
    }

    #[test]
    fn tail_preparation_rejects_exports_after_callback() {
        request_preparation_disconnect(PreparationCallback::Identity, false, true);
    }

    #[test]
    fn ordinary_preparation_rejects_question_after_descriptor_drop() {
        request_preparation_disconnect(PreparationCallback::Drop, false, false);
    }

    #[test]
    fn streaming_preparation_rejects_question_after_descriptor_drop() {
        request_preparation_disconnect(PreparationCallback::Drop, true, false);
    }

    #[test]
    fn preparation_checks_resolution_callback_before_export() {
        request_preparation_disconnect(PreparationCallback::Resolution, false, false);
    }

    #[test]
    fn export_resolution_preparation_can_disconnect_without_table_borrows() {
        let mut network = crate::twoparty::VatNetwork::new(
            PendingRead,
            futures::io::Cursor::new(Vec::<u8>::new()),
            Side::Client,
            Default::default(),
        );
        let (done, _completion) = oneshot::channel();
        let (mut tasks, state) = ConnectionState::new(
            broken::new_cap(Error::failed("unused bootstrap".into())),
            network.connect(Side::Server).unwrap(),
            done,
        );
        let id = state
            .exports
            .borrow_mut()
            .push(Export::new(broken::new_cap(Error::failed(
                "old promise".into(),
            ))));
        let resolution = preparation_hook(&state, PreparationCallback::Resolution);
        let operation =
            ConnectionState::resolve_exported_promise(&state, id, Promise::ok(resolution));
        assert!((&mut tasks).now_or_never().is_none());
        assert_preparation_error(
            operation
                .now_or_never()
                .expect("resolver must settle")
                .unwrap_err(),
        );
        assert_empty_bookkeeping(&state);
    }

    #[test]
    fn result_preparation_rejects_exports_after_callback() {
        let (_tasks, state, _completion) = connection();
        let (sender, pipeline) = queued::Pipeline::new();
        let (done, _receiver) = oneshot::channel();
        state.answers.borrow_mut().slots.insert(7, Answer::new());
        let mut results = Results::new(&state, 7, false, done, Rc::new(Cell::new(false)), None);
        results
            .get()
            .unwrap()
            .set_as_capability(preparation_hook(&state, PreparationCallback::Identity));
        let inner = results.inner.take().unwrap();
        results.results_done_fulfiller.take();
        let result = ResultsDone::from_results_inner(Ok(inner), Ok(()), sender);
        assert_empty_bookkeeping(&state);
        assert_preparation_error(result.err().expect("terminal results"));
        let cap = pipeline.get_pipelined_cap(&[]);
        assert_preparation_error(
            cap.new_call(0, 0, None)
                .send()
                .promise
                .now_or_never()
                .expect("terminal result pipeline")
                .err()
                .unwrap(),
        );
    }
    #[test]
    fn bootstrap_rejects_question_after_disconnect() {
        let (_tasks, state, _completion) = connection();
        state.disconnect(Error::disconnected("preparation stopped".into()));
        let cap = ConnectionState::bootstrap(&state);
        assert_empty_bookkeeping(&state);
        assert_preparation_error(
            cap.new_call(0, 0, None)
                .send()
                .promise
                .now_or_never()
                .expect("terminal bootstrap")
                .err()
                .unwrap(),
        );
    }
    #[test]
    fn descriptor_preparation_reuses_reentrant_export() {
        let (_tasks, state, _completion) = connection();
        let inner = Box::new(local::Client::new(DropServer(Rc::new(DropAction(
            Box::new(|| {}),
        ))))) as Box<dyn ClientHook>;
        let nested = inner.clone();
        let weak = Rc::downgrade(&state);
        let hook = PreparationHook {
            inner,
            callback: Rc::new(RefCell::new(Some(Box::new(move || {
                let mut message = capnp::message::Builder::new_default();
                ConnectionState::write_descriptor(
                    &weak.upgrade().unwrap(),
                    nested,
                    message.init_root(),
                )
                .unwrap();
            })))),
            stage: PreparationCallback::Resolution,
            is_copy: false,
        };
        let mut message = capnp::message::Builder::new_default();
        let id = ConnectionState::write_descriptor(&state, Box::new(hook), message.init_root())
            .unwrap()
            .unwrap();
        assert_eq!(state.exports.borrow().iter().count(), 1);
        assert_eq!(state.exports_by_cap.borrow().len(), 1);
        assert_eq!(state.exports.borrow_mut().find(id).unwrap().refcount, 2);
    }
}
