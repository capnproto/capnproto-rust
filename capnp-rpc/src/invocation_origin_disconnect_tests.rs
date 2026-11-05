use capnp::private::capability::InvocationOrigin;

#[test]
fn typed_origin_preserves_descriptor_disconnect_errors_and_cleanup() {
    for at in [
        DisconnectAt::Drop,
        DisconnectAt::GetPtr,
        DisconnectAt::AddRef,
        DisconnectAt::WhenMoreResolved,
    ] {
        check_tagged_disconnect_while_writing_params(at);
    }
}

#[test]
fn typed_conflict_does_not_mask_an_existing_disconnect() {
    for streaming in [false, true] {
        let (_tasks, state) = new_connection_state();
        let target = ConnectionState::import(&state, 17, false);
        let mut request = target.new_call(0, 0, None);
        request
            .hook
            .set_invocation_origin(InvocationOrigin::new())
            .unwrap();
        assert!(request
            .hook
            .set_invocation_origin(InvocationOrigin::new())
            .is_err());
        state.disconnect(Error::disconnected("original disconnect".into()));
        let result = if streaming {
            request.hook.send_streaming().now_or_never()
        } else {
            request.send().promise.map_ok(|_| ()).now_or_never()
        };
        assert_disconnected_with(result.unwrap().unwrap_err(), "original disconnect");
        assert_eq!(state.questions.borrow().iter().count(), 0);
        assert_eq!(state.exports.borrow().iter().count(), 0);
    }
}

struct DisconnectingOriginParams {
    state: Weak<ConnectionState<Side>>,
    message: capnp::message::Builder<capnp::message::HeapAllocator>,
}

impl ParamsHook for DisconnectingOriginParams {
    fn invocation_origin(&self) -> Option<InvocationOrigin> {
        self.state
            .upgrade()
            .unwrap()
            .disconnect(Error::disconnected("origin getter".into()));
        Some(InvocationOrigin::new())
    }
    fn get(&self) -> capnp::Result<any_pointer::Reader<'_>> {
        self.message.get_root_as_reader()
    }
}

struct UnusedResults;
impl ResultsHook for UnusedResults {
    fn get(&mut self) -> capnp::Result<any_pointer::Builder<'_>> {
        panic!("unexpected result")
    }
    fn set_pipeline(&mut self) -> capnp::Result<()> {
        panic!("unexpected pipeline")
    }
    fn allow_cancellation(&self) {
        panic!("unexpected cancellation change")
    }
    fn tail_call(self: Box<Self>, _: Box<dyn RequestHook>) -> Promise<(), Error> {
        panic!("unexpected tail call")
    }
    fn direct_tail_call(
        self: Box<Self>,
        _: Box<dyn RequestHook>,
    ) -> (Promise<(), Error>, Box<dyn PipelineHook>) {
        panic!("unexpected direct tail call")
    }
}

#[test]
fn typed_origin_getter_disconnect_cannot_admit_copied_request() {
    let (_tasks, state) = new_connection_state();
    let target = ConnectionState::import(&state, 17, false);
    let params = DisconnectingOriginParams {
        state: Rc::downgrade(&state),
        message: capnp::message::Builder::new_default(),
    };
    let error = target
        .call(0, 0, Box::new(params), Box::new(UnusedResults))
        .now_or_never()
        .unwrap()
        .unwrap_err();
    assert_disconnected_with(error, "origin getter");
    assert_eq!(state.questions.borrow().iter().count(), 0);
    assert_eq!(state.exports.borrow().iter().count(), 0);
}

#[derive(Clone)]
struct OriginRedirectTarget {
    state: Weak<ConnectionState<Side>>,
    sent: Rc<Cell<bool>>,
    close_in_setter: bool,
}

impl ClientHook for OriginRedirectTarget {
    fn add_ref(&self) -> Box<dyn ClientHook> {
        Box::new(self.clone())
    }
    fn new_call(
        &self,
        _: u64,
        _: u16,
        _: Option<capnp::MessageSize>,
    ) -> capnp::capability::Request<any_pointer::Owned, any_pointer::Owned> {
        capnp::capability::Request::new(Box::new(OriginRedirectRequest {
            target: self.clone(),
            origin: None,
            message: capnp::message::Builder::new_default(),
        }))
    }
    fn call(
        &self,
        _: u64,
        _: u16,
        _: Box<dyn ParamsHook>,
        _: Box<dyn ResultsHook>,
    ) -> Promise<(), Error> {
        panic!("redirect must preserve genuine request construction")
    }
    fn get_brand(&self) -> usize {
        0
    }
    fn get_ptr(&self) -> usize {
        Rc::as_ptr(&self.sent) as usize
    }
    fn get_resolved(&self) -> Option<Box<dyn ClientHook>> {
        None
    }
    fn when_more_resolved(&self) -> Option<Promise<Box<dyn ClientHook>, Error>> {
        None
    }
    fn when_resolved(&self) -> Promise<(), Error> {
        Promise::ok(())
    }
}

struct OriginRedirectRequest {
    target: OriginRedirectTarget,
    origin: Option<InvocationOrigin>,
    message: capnp::message::Builder<capnp::message::HeapAllocator>,
}

impl RequestHook for OriginRedirectRequest {
    fn set_invocation_origin(&mut self, origin: InvocationOrigin) -> capnp::Result<()> {
        self.origin = Some(origin);
        if self.target.close_in_setter {
            self.target
                .state
                .upgrade()
                .unwrap()
                .disconnect(Error::disconnected("origin setter".into()));
            // The causal disconnect must win over this subsequent wrapper error.
            return Err(Error::failed("setter error after disconnect".into()));
        }
        Ok(())
    }
    fn get(&mut self) -> any_pointer::Builder<'_> {
        assert!(
            self.origin.is_some(),
            "origin must be assigned before copying"
        );
        if !self.target.close_in_setter {
            self.target
                .state
                .upgrade()
                .unwrap()
                .disconnect(Error::disconnected("replacement copy".into()));
        }
        self.message.get_root().unwrap()
    }
    fn get_brand(&self) -> usize {
        0
    }
    fn send(self: Box<Self>) -> capnp::capability::RemotePromise<any_pointer::Owned> {
        self.target.sent.set(true);
        Box::new(broken::Request::new(
            Error::failed("unexpected local send".into()),
            None,
        ))
        .send()
    }
    fn send_streaming(self: Box<Self>) -> Promise<(), Error> {
        self.target.sent.set(true);
        Promise::err(Error::failed("unexpected local streaming send".into()))
    }
    fn tail_send(self: Box<Self>) -> Option<(u32, Promise<(), Error>, Box<dyn PipelineHook>)> {
        panic!("unexpected tail send")
    }
}

#[test]
fn typed_redirect_callback_disconnect_prevents_local_send() {
    for streaming in [false, true] {
        for close_in_setter in [false, true] {
            let (_tasks, state) = new_connection_state();
            let import = ConnectionState::import(&state, 17, false);
            let pending = PromiseClient::new(&state, import, None);
            let target = Client::from(pending.clone());
            let mut request = target.new_call(0, 0, None);
            request
                .hook
                .set_invocation_origin(InvocationOrigin::new())
                .unwrap();
            let sent = Rc::new(Cell::new(false));
            pending
                .borrow_mut()
                .resolve(Ok(Box::new(OriginRedirectTarget {
                    state: Rc::downgrade(&state),
                    sent: sent.clone(),
                    close_in_setter,
                })));
            let result = if streaming {
                request.hook.send_streaming().now_or_never()
            } else {
                request.send().promise.map_ok(|_| ()).now_or_never()
            };
            assert_disconnected_with(
                result.unwrap().unwrap_err(),
                if close_in_setter {
                    "origin setter"
                } else {
                    "replacement copy"
                },
            );
            assert!(!sent.get(), "closed source connection admitted local work");
            assert_eq!(state.questions.borrow().iter().count(), 0);
            assert_eq!(state.exports.borrow().iter().count(), 0);
        }
    }
}

fn check_tagged_disconnect_while_writing_params(disconnect_at: DisconnectAt) {
    for kind in [
        SendKind::Ordinary,
        SendKind::Streaming,
        SendKind::StreamingWithController,
        SendKind::Tail,
    ] {
        let (_tasks, state) = new_connection_state();
        let target = ConnectionState::import(&state, 17, false);
        if matches!(kind, SendKind::StreamingWithController) {
            let client = Client::from_ptr(target.get_ptr(), &state).unwrap();
            *client.flow_controller.borrow_mut() = Some(Box::new(UnusedFlowController));
        }
        let inner = if disconnect_at == DisconnectAt::Drop {
            ConnectionState::import(&state, 23, false)
        } else {
            broken::new_cap(Error::failed("unused local cap".into()))
        };
        let cap = DisconnectingHook::new(&state, inner, disconnect_at);
        let observed = cap.observed.clone();
        if disconnect_at == DisconnectAt::GetPtr {
            // The disconnect must clear a previously exported capability,
            // without the resumed descriptor path recreating its export.
            let ptr = cap.get_ptr();
            let export_id = state.exports.borrow_mut().push(Export::new(cap.add_ref()));
            state.exports_by_cap.borrow_mut().insert(ptr, export_id);
        }
        let mut request = target.new_call(0, 0, None);
        request
            .hook
            .set_invocation_origin(InvocationOrigin::new())
            .unwrap();
        let mut params = request
            .get()
            .initn_as::<capnp::any_pointer_list::Builder>(2);
        params.reborrow().get(0).set_as_capability(Box::new(cap));
        // A later descriptor must not create an export after the first
        // descriptor's callback has completed disconnect cleanup.
        params
            .get(1)
            .set_as_capability(broken::new_cap(Error::failed("later cap".into())));
        assert!(state.connection.borrow().is_ok());
        match kind {
            SendKind::Ordinary => {
                let result = request.send();
                assert_eq!(state.questions.borrow().iter().count(), 0, "{kind:?}");
                expect_serialization_disconnect(result.promise.now_or_never());
                let pipelined = result.pipeline.as_cap().new_call(0, 0, None).send();
                expect_serialization_disconnect(pipelined.promise.now_or_never());
            }
            SendKind::Streaming | SendKind::StreamingWithController => {
                let result = request.hook.send_streaming();
                assert_eq!(state.questions.borrow().iter().count(), 0, "{kind:?}");
                expect_serialization_disconnect(result.now_or_never());
            }
            SendKind::Tail => assert!(request.hook.tail_send().is_none()),
        }
        assert_cleaned_up(&state, observed);
    }
}

struct CountedIngressConnection {
    inner: Box<dyn crate::Connection<Side>>,
    origin: InvocationOrigin,
    reads: Rc<Cell<usize>>,
    published: Rc<Cell<bool>>,
}

impl crate::Connection<Side> for CountedIngressConnection {
    fn incoming_invocation_origin(&self) -> Option<InvocationOrigin> {
        assert!(
            !self.published.get(),
            "classification read after state publication"
        );
        self.reads.set(self.reads.get() + 1);
        Some(self.origin.clone())
    }
    fn get_peer_vat_id(&self) -> Side {
        self.inner.get_peer_vat_id()
    }
    fn new_outgoing_message(&mut self, words: u32) -> Box<dyn crate::OutgoingMessage> {
        self.inner.new_outgoing_message(words)
    }
    fn receive_incoming_message(
        &mut self,
    ) -> Promise<Option<Box<dyn crate::IncomingMessage>>, Error> {
        Promise::from_future(future::pending())
    }
    fn shutdown(&mut self, result: capnp::Result<()>) -> Promise<(), Error> {
        self.inner.shutdown(result)
    }
}

struct OriginIncomingMessage(capnp::message::Builder<capnp::message::HeapAllocator>);
impl crate::IncomingMessage for OriginIncomingMessage {
    fn get_body(&self) -> capnp::Result<any_pointer::Reader<'_>> {
        self.0.get_root_as_reader()
    }
}

#[test]
fn typed_ingress_origin_is_captured_once_before_state_publication() {
    // protects: received calls retain trusted ingress identity in their Params.
    // break: removing the cached-origin transfer dispatches missing origins.
    // observe: both received calls record the connection's identity and payload.
    // seam: none; use the existing dispatch and connection hooks.
    let mut network = crate::twoparty::VatNetwork::new(
        futures::io::Cursor::new(Vec::<u8>::new()),
        futures::io::Cursor::new(Vec::<u8>::new()),
        Side::Client,
        Default::default(),
    );
    let reads = Rc::new(Cell::new(0));
    let published = Rc::new(Cell::new(false));
    let origin = InvocationOrigin::new();
    let connection = CountedIngressConnection {
        inner: network.connect(Side::Server).unwrap(),
        origin: origin.clone(),
        reads: reads.clone(),
        published: published.clone(),
    };
    let (disconnect, _disconnected) = oneshot::channel();
    let (mut tasks, state) = ConnectionState::new(
        broken::new_cap(Error::failed("unused bootstrap".into())),
        Box::new(connection),
        disconnect,
    );
    published.set(true);
    assert_eq!(reads.get(), 1);
    assert_eq!(state.incoming_invocation_origin.as_ref(), Some(&origin));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let export = state
        .exports
        .borrow_mut()
        .push(Export::new(Box::new(local::Client::new(OriginRecorder(
            seen.clone(),
        )))));
    for question_id in 0..2 {
        let mut incoming = capnp::message::Builder::new_default();
        let mut call = incoming.init_root::<message::Builder>().init_call();
        call.set_question_id(question_id);
        call.reborrow().init_target().set_imported_cap(export);
        call.reborrow().init_params().init_cap_table(0);
        call.reborrow()
            .get_params()
            .unwrap()
            .get_content()
            .set_as::<capnp::text::Owned>("payload")
            .unwrap();
        call.init_send_results_to().set_caller(());
        ConnectionState::handle_message(
            &Rc::downgrade(&state),
            Box::new(OriginIncomingMessage(incoming)),
        )
        .unwrap();
    }
    assert!((&mut tasks).now_or_never().is_none());
    assert_eq!(*seen.borrow(), [origin.clone(), origin]);
    assert_eq!(
        reads.get(),
        1,
        "incoming dispatch must use the cached identity"
    );
    state.disconnect(Error::disconnected("done".into()));
    assert_eq!(reads.get(), 1);
}
