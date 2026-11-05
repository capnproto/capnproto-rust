// These tests use private hooks to select reconstruction and redirect boundaries
// deterministically. End-to-end transport scheduling is tested downstream.

fn complete_origin_call<F: futures::Future>(call: F) -> F::Output {
    call.now_or_never()
        .expect("local call must finish in one poll; no wire response is available")
}

#[derive(Clone)]
struct OriginRecorder(Rc<RefCell<Vec<InvocationOrigin>>>);

impl capnp::capability::Server for OriginRecorder {
    fn dispatch_call(
        self,
        _: u64,
        method: u16,
        params: capnp::capability::Params<any_pointer::Owned>,
        mut results: capnp::capability::Results<any_pointer::Owned>,
    ) -> capnp::capability::DispatchCallResult {
        let result = (|| {
            if method == 1 {
                return Err(Error::failed("denied method".into()));
            }
            let origin = params
                .hook
                .invocation_origin()
                .ok_or_else(|| Error::failed("missing origin".into()))?;
            assert_eq!(
                params.get()?.get_as::<capnp::text::Reader<'_>>()?,
                "payload"
            );
            self.0.borrow_mut().push(origin);
            if method == 3 {
                results
                    .get()
                    .set_as_capability(Box::new(local::Client::new(self.clone())));
            }
            Ok(())
        })();
        capnp::capability::DispatchCallResult::new(
            Promise::from_future(async { result }),
            method == 2,
        )
    }
    fn as_ptr(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
}

struct CopiedOriginParams {
    message: capnp::message::Builder<capnp::message::HeapAllocator>,
    origin: InvocationOrigin,
    resolve: RefCell<Option<(Rc<RefCell<PromiseClient<Side>>>, Box<dyn ClientHook>)>>,
    copied: Rc<Cell<bool>>,
}
impl ParamsHook for CopiedOriginParams {
    fn get(&self) -> capnp::Result<any_pointer::Reader<'_>> {
        // Resolve independently of provenance transfer. Client::new_call still
        // constructs a genuine RPC request; sending it redirects to this grant.
        // Removing origin transfer must fail for missing origin, not routing.
        if let Some((pending, target)) = self.resolve.borrow_mut().take() {
            pending.borrow_mut().resolve(Ok(target));
        }
        self.message.get_root_as_reader()
    }
    fn invocation_origin(&self) -> Option<InvocationOrigin> {
        self.copied.set(true);
        Some(self.origin.clone())
    }
}

struct EmptyResults(capnp::message::Builder<capnp::message::HeapAllocator>);
impl ResultsHook for EmptyResults {
    fn get(&mut self) -> capnp::Result<any_pointer::Builder<'_>> {
        self.0.get_root()
    }
    fn set_pipeline(&mut self) -> capnp::Result<()> {
        Ok(())
    }
    fn allow_cancellation(&self) {}
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

fn tagged_request(
    target: &dyn ClientHook,
    origin: &InvocationOrigin,
    method: u16,
) -> capnp::capability::Request<any_pointer::Owned, any_pointer::Owned> {
    let mut request = target.new_call(0, method, None);
    request.hook.set_invocation_origin(origin.clone()).unwrap();
    request
        .get()
        .set_as::<capnp::text::Owned>("payload")
        .unwrap();
    request
}

#[test]
fn typed_delegated_origin_survives_private_copy_and_redirect() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let grant = local::Client::new(OriginRecorder(seen.clone()));
    let independent = local::Client::new(OriginRecorder(seen.clone()));
    let a = InvocationOrigin::new();
    let b = InvocationOrigin::new();
    complete_origin_call(tagged_request(&grant, &a, 0).send().promise).unwrap();
    let (_tasks, state) = new_connection_state();
    let pending = PromiseClient::new(&state, ConnectionState::import(&state, 17, false), None);
    let imported = Client::from(pending.clone());
    let mut message = capnp::message::Builder::new_default();
    message.set_root::<capnp::text::Owned>("payload").unwrap();
    let copied = Rc::new(Cell::new(false));
    let call = imported.call(
        0,
        0,
        Box::new(CopiedOriginParams {
            message,
            origin: b.clone(),
            resolve: RefCell::new(Some((pending.clone(), grant.add_ref()))),
            copied: copied.clone(),
        }),
        Box::new(EmptyResults(capnp::message::Builder::new_default())),
    );
    complete_origin_call(call).unwrap();
    assert!(copied.get(), "private-copy accessor did not run");
    assert_eq!(pending.borrow().cap.get_ptr(), grant.get_ptr());
    assert_eq!(pending.borrow().cap.get_brand(), grant.get_brand());
    assert_eq!(
        state.questions.borrow().iter().count(),
        0,
        "redirect emitted a wire question"
    );
    complete_origin_call(tagged_request(&independent, &b, 0).send().promise).unwrap();
    assert_eq!(*seen.borrow(), [a, b.clone(), b]);
    let denied = complete_origin_call(
        tagged_request(&grant, &InvocationOrigin::new(), 1)
            .send()
            .promise,
    );
    assert_eq!(denied.err().unwrap().extra, "denied method");
    assert_eq!(seen.borrow().len(), 3);
    state.disconnect(Error::disconnected("done".into()));
}

#[test]
fn typed_ordinary_and_streaming_redirects_preserve_origin() {
    for streaming in [false, true] {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let grant = local::Client::new(OriginRecorder(seen.clone()));
        let origin = InvocationOrigin::new();
        let (_tasks, state) = new_connection_state();
        let pending = PromiseClient::new(&state, ConnectionState::import(&state, 17, false), None);
        let imported = Client::from(pending.clone());
        let request = tagged_request(&imported, &origin, if streaming { 2 } else { 0 });
        pending.borrow_mut().resolve(Ok(grant.add_ref()));
        if streaming {
            complete_origin_call(request.hook.send_streaming()).unwrap();
        } else {
            complete_origin_call(request.send().promise).unwrap();
        }
        assert_eq!(*seen.borrow(), [origin]);
        assert_eq!(state.questions.borrow().iter().count(), 0);
        state.disconnect(Error::disconnected("done".into()));
    }
}

#[test]
fn typed_conflict_remains_poisoned_when_setter_errors_are_ignored() {
    for streaming in [false, true] {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let grant = local::Client::new(OriginRecorder(seen.clone()));
        let origin = InvocationOrigin::new();
        let mut request = tagged_request(&grant, &origin, if streaming { 2 } else { 0 });
        request.hook.set_invocation_origin(origin.clone()).unwrap();
        assert!(request
            .hook
            .set_invocation_origin(InvocationOrigin::new())
            .is_err());
        assert!(request.hook.set_invocation_origin(origin).is_err());
        let error = if streaming {
            complete_origin_call(request.hook.send_streaming()).unwrap_err()
        } else {
            complete_origin_call(request.send().promise).err().unwrap()
        };
        assert_eq!(error.extra, "conflicting local call context");
        assert!(seen.borrow().is_empty());
    }
}

// protects: queued calls retain their own origins through capability resolution.
// break: losing Params provenance or inheriting the parent pipeline's origin.
// observe: delayed calls record A then B through the same returned capability.
// seam: none.
#[test]
fn typed_queued_pipeline_calls_keep_independent_origins() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let (resolve, pending) = oneshot::channel::<Box<dyn ClientHook>>();
    let client: Box<dyn ClientHook> =
        crate::new_future_client(pending.map_err(crate::canceled_to_error));
    let a = InvocationOrigin::new();
    let b = InvocationOrigin::new();
    let first = tagged_request(client.as_ref(), &a, 3).send();
    let pipelined = first.pipeline.as_cap();
    let second = tagged_request(pipelined.as_ref(), &b, 0).send();
    let mut calls = Box::pin(futures::future::try_join(first.promise, second.promise));
    assert!(calls.as_mut().now_or_never().is_none());
    assert!(seen.borrow().is_empty());
    drop(client);
    drop(pipelined);
    assert!(resolve
        .send(Box::new(local::Client::new(OriginRecorder(seen.clone()))))
        .is_ok());
    complete_origin_call(calls).unwrap();
    assert_eq!(*seen.borrow(), [a, b]);
}

// protects: reconnect request wrappers preserve origin and conflict rejection.
// break: omitting setter delegation or swallowing its conflict error.
// observe: ordinary calls record their caller; a poisoned call never dispatches.
// seam: none.
#[test]
fn typed_reconnect_requests_preserve_origin_and_conflict() {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let recorder = seen.clone();
    let (client, _set_target) = crate::auto_reconnect(move || {
        Ok(Box::new(local::Client::new(OriginRecorder(recorder.clone()))) as Box<dyn ClientHook>)
    })
    .unwrap();
    let a = InvocationOrigin::new();
    let b = InvocationOrigin::new();
    complete_origin_call(tagged_request(client.as_ref(), &a, 0).send().promise).unwrap();
    complete_origin_call(tagged_request(client.as_ref(), &b, 0).send().promise).unwrap();
    let mut conflicted = tagged_request(client.as_ref(), &a, 0);
    assert!(conflicted.hook.set_invocation_origin(b.clone()).is_err());
    assert!(complete_origin_call(conflicted.send().promise).is_err());
    assert_eq!(*seen.borrow(), [a, b]);
}

// protects: identity-keyed native registries distinguish fresh tokens from clones.
// break: value equality for fresh tokens or inconsistent clone hashing.
// observe: a retained clone finds its original entry; a fresh token does not.
// seam: none.
#[test]
fn typed_origin_clones_preserve_registry_identity() {
    let origin = InvocationOrigin::new();
    let clone = origin.clone();
    let fresh = InvocationOrigin::new();
    let mut registry = std::collections::HashSet::new();
    assert!(registry.insert(origin));
    assert!(!registry.insert(clone.clone()));
    assert!(registry.contains(&clone));
    assert!(!registry.contains(&fresh));
    assert!(registry.insert(fresh));
    assert_eq!(registry.len(), 2);
}

struct UntaggedParams(capnp::message::Builder<capnp::message::HeapAllocator>);
impl ParamsHook for UntaggedParams {
    fn get(&self) -> capnp::Result<any_pointer::Reader<'_>> {
        self.0.get_root_as_reader()
    }
}

struct UnsupportedOriginRequest(Box<dyn RequestHook>);
impl RequestHook for UnsupportedOriginRequest {
    fn get(&mut self) -> any_pointer::Builder<'_> {
        self.0.get()
    }
    fn get_brand(&self) -> usize {
        self.0.get_brand()
    }
    fn send(self: Box<Self>) -> capnp::capability::RemotePromise<any_pointer::Owned> {
        self.0.send()
    }
    fn send_streaming(self: Box<Self>) -> Promise<(), Error> {
        self.0.send_streaming()
    }
    fn tail_send(self: Box<Self>) -> Option<(u32, Promise<(), Error>, Box<dyn PipelineHook>)> {
        self.0.tail_send()
    }
}

// protects: old hook implementations do not silently claim origin support.
// break: accepting unsupported transfer or assigning an origin to untagged calls.
// observe: default hooks report absence/Unimplemented; the origin-dependent sink rejects.
// seam: none.
#[test]
fn typed_origin_defaults_reject_unsupported_or_absent_provenance() {
    let params = UntaggedParams(capnp::message::Builder::new_default());
    assert!(params.invocation_origin().is_none());
    let seen = Rc::new(RefCell::new(Vec::new()));
    let target = local::Client::new(OriginRecorder(seen.clone()));
    let mut request = target.new_call(0, 0, None);
    request
        .get()
        .set_as::<capnp::text::Owned>("payload")
        .unwrap();
    let mut unsupported = UnsupportedOriginRequest(request.hook);
    assert_eq!(
        unsupported
            .set_invocation_origin(InvocationOrigin::new())
            .unwrap_err()
            .kind,
        capnp::ErrorKind::Unimplemented
    );
    drop(unsupported);
    let error = complete_origin_call(target.new_call(0, 0, None).send().promise)
        .err()
        .unwrap();
    assert_eq!(error.extra, "missing origin");
    assert!(seen.borrow().is_empty());
}
