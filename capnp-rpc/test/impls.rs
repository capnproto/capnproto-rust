// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors
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

use crate::test_capnp::{
    bootstrap, test_call_order, test_capability_server_set, test_extends, test_handle,
    test_interface, test_more_stuff, test_pipeline, test_promise_resolve, test_self,
    test_streaming,
};

use capnp::capability::FromClientHook;
use capnp::Error;

use futures::channel::oneshot;
use futures::TryFutureExt;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Bootstrap;

impl bootstrap::Server for Bootstrap {
    async fn test_interface(
        self: Rc<Self>,
        _params: bootstrap::TestInterfaceParams,
        mut results: bootstrap::TestInterfaceResults,
    ) -> Result<(), Error> {
        results
            .get()
            .set_cap(capnp_rpc::new_client(TestInterface::new()));
        Ok(())
    }

    async fn test_extends(
        self: Rc<Self>,
        _params: bootstrap::TestExtendsParams,
        mut results: bootstrap::TestExtendsResults,
    ) -> Result<(), Error> {
        results.get().set_cap(capnp_rpc::new_client(TestExtends));
        Ok(())
    }

    async fn test_extends2(
        self: Rc<Self>,
        _params: bootstrap::TestExtends2Params,
        _results: bootstrap::TestExtends2Results,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    async fn test_pipeline(
        self: Rc<Self>,
        _params: bootstrap::TestPipelineParams,
        mut results: bootstrap::TestPipelineResults,
    ) -> Result<(), Error> {
        results.get().set_cap(capnp_rpc::new_client(TestPipeline));
        Ok(())
    }

    async fn test_call_order(
        self: Rc<Self>,
        _params: bootstrap::TestCallOrderParams,
        mut results: bootstrap::TestCallOrderResults,
    ) -> Result<(), Error> {
        {
            results
                .get()
                .set_cap(capnp_rpc::new_client(TestCallOrder::new()));
        }
        Ok(())
    }
    async fn test_more_stuff(
        self: Rc<Self>,
        _params: bootstrap::TestMoreStuffParams,
        mut results: bootstrap::TestMoreStuffResults,
    ) -> Result<(), Error> {
        {
            results
                .get()
                .set_cap(capnp_rpc::new_client(TestMoreStuff::new()));
        }
        Ok(())
    }
    async fn test_capability_server_set(
        self: Rc<Self>,
        _params: bootstrap::TestCapabilityServerSetParams,
        mut results: bootstrap::TestCapabilityServerSetResults,
    ) -> Result<(), Error> {
        results
            .get()
            .set_cap(capnp_rpc::new_client(TestCapabilityServerSet::new()));
        Ok(())
    }

    async fn test_promise_resolve(
        self: Rc<Self>,
        _params: bootstrap::TestPromiseResolveParams,
        mut results: bootstrap::TestPromiseResolveResults,
    ) -> Result<(), Error> {
        results
            .get()
            .set_cap(capnp_rpc::new_client(TestPromiseResolveImpl {}));
        Ok(())
    }
}

#[derive(Default)]
pub struct TestInterface {
    call_count: Rc<Cell<u64>>,
}

impl TestInterface {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get_call_count(&self) -> Rc<Cell<u64>> {
        self.call_count.clone()
    }
    fn increment_call_count(&self) {
        self.call_count.set(self.call_count.get() + 1);
    }
}

impl test_interface::Server for TestInterface {
    async fn foo(
        self: Rc<Self>,
        params: test_interface::FooParams,
        mut results: test_interface::FooResults,
    ) -> Result<(), Error> {
        self.increment_call_count();
        let params = params.get()?;
        if params.get_i() != 123 {
            return Err(Error::failed("expected i to equal 123".to_string()));
        }
        if !params.get_j() {
            return Err(Error::failed("expected j to be true".to_string()));
        }

        results.get().set_x("foo");

        Ok(())
    }

    async fn bar(
        self: Rc<Self>,
        _params: test_interface::BarParams,
        _results: test_interface::BarResults,
    ) -> Result<(), Error> {
        self.increment_call_count();
        Err(Error::unimplemented("bar is not implemented".to_string()))
    }

    async fn baz(
        self: Rc<Self>,
        params: test_interface::BazParams,
        _results: test_interface::BazResults,
    ) -> Result<(), Error> {
        self.increment_call_count();
        crate::test_util::CheckTestMessage::check_test_message(params.get()?.get_s()?);
        Ok(())
    }
}

struct TestExtends;

impl test_interface::Server for TestExtends {
    async fn foo(
        self: Rc<Self>,
        params: test_interface::FooParams,
        mut results: test_interface::FooResults,
    ) -> Result<(), Error> {
        let params = params.get()?;
        if params.get_i() != 321 {
            return Err(Error::failed("expected i to equal 321".to_string()));
        }
        if params.get_j() {
            return Err(Error::failed("expected j to be false".to_string()));
        }
        {
            let mut results = results.get();
            results.set_x("bar");
        }
        Ok(())
    }

    async fn bar(
        self: Rc<Self>,
        _params: test_interface::BarParams,
        _results: test_interface::BarResults,
    ) -> Result<(), Error> {
        Err(Error::unimplemented("bar is not implemented".to_string()))
    }

    async fn baz(
        self: Rc<Self>,
        _params: test_interface::BazParams,
        _results: test_interface::BazResults,
    ) -> Result<(), Error> {
        Err(Error::unimplemented("baz is not implemented".to_string()))
    }
}

impl test_extends::Server for TestExtends {
    async fn qux(
        self: Rc<Self>,
        _params: test_extends::QuxParams,
        _results: test_extends::QuxResults,
    ) -> Result<(), Error> {
        Err(Error::unimplemented("qux is not implemented".to_string()))
    }

    async fn corge(
        self: Rc<Self>,
        _params: test_extends::CorgeParams,
        _results: test_extends::CorgeResults,
    ) -> Result<(), Error> {
        Err(Error::unimplemented("corge is not implemented".to_string()))
    }

    async fn grault(
        self: Rc<Self>,
        _params: test_extends::GraultParams,
        mut results: test_extends::GraultResults,
    ) -> Result<(), Error> {
        crate::test_util::init_test_message(results.get());
        Ok(())
    }
}

struct TestPipeline;

impl test_pipeline::Server for TestPipeline {
    async fn get_cap(
        self: Rc<Self>,
        params: test_pipeline::GetCapParams,
        mut results: test_pipeline::GetCapResults,
    ) -> Result<(), Error> {
        if params.get()?.get_n() != 234 {
            return Err(Error::failed("expected n to equal 234".to_string()));
        }
        let cap = params.get()?.get_in_cap()?;
        let mut request = cap.foo_request();
        request.get().set_i(123);
        request.get().set_j(true);

        let response = request.send().promise.await;
        if response?.get()?.get_x()? != "foo" {
            return Err(Error::failed("expected x to equal 'foo'".to_string()));
        }

        results.get().set_s("bar");

        results
            .get()
            .init_out_box()
            .set_cap(capnp_rpc::new_client::<test_extends::Client, _>(TestExtends).cast_to());
        Ok(())
    }

    async fn get_null_cap(
        self: Rc<Self>,
        _params: test_pipeline::GetNullCapParams,
        _results: test_pipeline::GetNullCapResults,
    ) -> Result<(), Error> {
        Ok(())
    }

    async fn get_cap_pipeline_only(
        self: Rc<Self>,
        _params: test_pipeline::GetCapPipelineOnlyParams,
        mut results: test_pipeline::GetCapPipelineOnlyResults,
    ) -> Result<(), Error> {
        results
            .get()
            .init_out_box()
            .set_cap(capnp_rpc::new_client::<test_extends::Client, _>(TestExtends).cast_to());
        results.set_pipeline()?;
        ::futures::future::pending().await
    }
}

#[derive(Default)]
pub struct TestCallOrder {
    count: Cell<u32>,
}

impl TestCallOrder {
    pub fn new() -> Self {
        Self::default()
    }
}

impl test_call_order::Server for TestCallOrder {
    async fn get_call_sequence(
        self: Rc<Self>,
        _params: test_call_order::GetCallSequenceParams,
        mut results: test_call_order::GetCallSequenceResults,
    ) -> Result<(), Error> {
        results.get().set_n(self.count.get());
        self.count.set(self.count.get() + 1);
        Ok(())
    }
}

#[derive(Default)]
pub struct TestMoreStuff {
    call_count: Cell<u32>,
    handle_count: Rc<Cell<i64>>,
    client_to_hold: RefCell<Option<test_interface::Client>>,
}

impl TestMoreStuff {
    pub fn new() -> Self {
        Self::default()
    }
}

impl test_call_order::Server for TestMoreStuff {
    async fn get_call_sequence(
        self: Rc<Self>,
        _params: test_call_order::GetCallSequenceParams,
        mut results: test_call_order::GetCallSequenceResults,
    ) -> Result<(), Error> {
        results.get().set_n(self.call_count.get());
        self.call_count.set(self.call_count.get() + 1);
        Ok(())
    }
}

impl test_more_stuff::Server for TestMoreStuff {
    async fn call_foo(
        self: Rc<Self>,
        params: test_more_stuff::CallFooParams,
        mut results: test_more_stuff::CallFooResults,
    ) -> Result<(), Error> {
        self.call_count.set(self.call_count.get() + 1);
        let cap = params.get()?.get_cap()?;
        let mut request = cap.foo_request();
        request.get().set_i(123);
        request.get().set_j(true);

        let response = request.send().promise.await?;
        if response.get()?.get_x()? != "foo" {
            return Err(Error::failed("expected x to equal 'foo'".to_string()));
        }
        results.get().set_s("bar");
        Ok(())
    }

    async fn call_foo_when_resolved(
        self: Rc<Self>,
        params: test_more_stuff::CallFooWhenResolvedParams,
        mut results: test_more_stuff::CallFooWhenResolvedResults,
    ) -> Result<(), Error> {
        self.call_count.set(self.call_count.get() + 1);
        let cap = params.get()?.get_cap()?;
        cap.client.when_resolved().await?;

        let mut request = cap.foo_request();
        request.get().set_i(123);
        request.get().set_j(true);
        let response = request.send().promise.await?;
        if response.get()?.get_x()? != "foo" {
            return Err(Error::failed("expected x to equal 'foo'".to_string()));
        }
        results.get().set_s("bar");
        Ok(())
    }

    async fn never_return(
        self: Rc<Self>,
        params: test_more_stuff::NeverReturnParams,
        mut results: test_more_stuff::NeverReturnResults,
    ) -> Result<(), Error> {
        self.call_count.set(self.call_count.get() + 1);

        let cap = params.get()?.get_cap()?;

        // Keep a clone alive to ensure it gets release when the promise is dropped.
        let _attached = cap.clone();

        // Also attach `cap` to the result struct so we can make sure that the results are released.
        results.get().set_cap_copy(cap);

        ::futures::future::pending().await
    }

    async fn hold(
        self: Rc<Self>,
        params: test_more_stuff::HoldParams,
        _results: test_more_stuff::HoldResults,
    ) -> Result<(), Error> {
        self.call_count.set(self.call_count.get() + 1);
        *self.client_to_hold.borrow_mut() = Some(params.get()?.get_cap()?);
        Ok(())
    }

    async fn dont_hold(
        self: Rc<Self>,
        params: test_more_stuff::DontHoldParams,
        _results: test_more_stuff::DontHoldResults,
    ) -> Result<(), Error> {
        self.call_count.set(self.call_count.get() + 1);
        let _ = Some(params.get()?.get_cap()?);
        Ok(())
    }

    async fn call_held(
        self: Rc<Self>,
        _params: test_more_stuff::CallHeldParams,
        mut results: test_more_stuff::CallHeldResults,
    ) -> Result<(), Error> {
        self.call_count.set(self.call_count.get() + 1);
        match &*self.client_to_hold.borrow() {
            None => Err(Error::failed("no held client".to_string())),
            Some(client) => {
                let mut request = client.foo_request();
                {
                    let mut params = request.get();
                    params.set_i(123);
                    params.set_j(true);
                }
                let response = request.send().promise.await?;
                if response.get()?.get_x()? != "foo" {
                    Err(Error::failed("expected X to equal 'foo'".to_string()))
                } else {
                    results.get().set_s("bar");
                    Ok(())
                }
            }
        }
    }

    async fn get_held(
        self: Rc<Self>,
        _params: test_more_stuff::GetHeldParams,
        mut results: test_more_stuff::GetHeldResults,
    ) -> Result<(), Error> {
        self.call_count.set(self.call_count.get() + 1);
        match &*self.client_to_hold.borrow() {
            None => Err(Error::failed("no held client".to_string())),
            Some(client) => {
                results.get().set_cap(client.clone());
                Ok(())
            }
        }
    }

    async fn echo(
        self: Rc<Self>,
        params: test_more_stuff::EchoParams,
        mut results: test_more_stuff::EchoResults,
    ) -> Result<(), Error> {
        self.call_count.set(self.call_count.get() + 1);
        results.get().set_cap(params.get()?.get_cap()?);
        Ok(())
    }

    async fn expect_cancel(
        self: Rc<Self>,
        _params: test_more_stuff::ExpectCancelParams,
        _results: test_more_stuff::ExpectCancelResults,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    async fn get_handle(
        self: Rc<Self>,
        _params: test_more_stuff::GetHandleParams,
        mut results: test_more_stuff::GetHandleResults,
    ) -> Result<(), Error> {
        self.call_count.set(self.call_count.get() + 1);
        let handle = Handle::new(&self.handle_count);
        results.get().set_handle(capnp_rpc::new_client(handle));
        Ok(())
    }

    async fn get_handle_count(
        self: Rc<Self>,
        _params: test_more_stuff::GetHandleCountParams,
        mut results: test_more_stuff::GetHandleCountResults,
    ) -> Result<(), Error> {
        self.call_count.set(self.call_count.get() + 1);
        results.get().set_count(self.handle_count.get());
        Ok(())
    }

    async fn get_null(
        self: Rc<Self>,
        _params: test_more_stuff::GetNullParams,
        _results: test_more_stuff::GetNullResults,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    async fn method_with_defaults(
        self: Rc<Self>,
        _params: test_more_stuff::MethodWithDefaultsParams,
        _results: test_more_stuff::MethodWithDefaultsResults,
    ) -> Result<(), Error> {
        unimplemented!()
    }

    async fn call_each_capability(
        self: Rc<Self>,
        params: test_more_stuff::CallEachCapabilityParams,
        _results: test_more_stuff::CallEachCapabilityResults,
    ) -> Result<(), Error> {
        let mut results = Vec::new();
        for cap in params.get()?.get_caps()? {
            let mut request = cap?.foo_request();
            request.get().set_i(123);
            request.get().set_j(true);
            results.push(request.send().promise);
        }

        ::futures::future::try_join_all(results).await?;
        Ok(())
    }

    async fn get_test_streaming(
        self: Rc<Self>,
        _params: test_more_stuff::GetTestStreamingParams,
        mut results: test_more_stuff::GetTestStreamingResults,
    ) -> Result<(), Error> {
        results
            .get()
            .set_cap(capnp_rpc::new_client(TestStreamingImpl::new()));
        Ok(())
    }

    async fn get_test_self(
        self: Rc<Self>,
        _params: test_more_stuff::GetTestSelfParams,
        mut results: test_more_stuff::GetTestSelfResults,
    ) -> Result<(), Error> {
        results
            .get()
            .set_cap(capnp_rpc::new_client(TestSelfImpl::new()));
        Ok(())
    }
}

struct Handle {
    count: Rc<Cell<i64>>,
}

impl Handle {
    fn new(count: &Rc<Cell<i64>>) -> Self {
        let count = count.clone();
        count.set(count.get() + 1);
        Self { count }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.count.set(self.count.get() - 1);
    }
}

impl test_handle::Server for Handle {}

pub struct TestCapDestructor {
    fulfiller: Option<oneshot::Sender<()>>,
    imp: Rc<TestInterface>,
}

impl TestCapDestructor {
    pub fn new(fulfiller: oneshot::Sender<()>) -> Self {
        Self {
            fulfiller: Some(fulfiller),
            imp: Rc::new(TestInterface::new()),
        }
    }
}

impl Drop for TestCapDestructor {
    fn drop(&mut self) {
        if let Some(f) = self.fulfiller.take() {
            let _ = f.send(());
        }
    }
}

impl test_interface::Server for TestCapDestructor {
    async fn foo(
        self: Rc<Self>,
        params: test_interface::FooParams,
        results: test_interface::FooResults,
    ) -> Result<(), Error> {
        self.imp.clone().foo(params, results).await
    }

    async fn bar(
        self: Rc<Self>,
        _params: test_interface::BarParams,
        _results: test_interface::BarResults,
    ) -> Result<(), Error> {
        Err(Error::unimplemented("bar is not implemented".to_string()))
    }

    async fn baz(
        self: Rc<Self>,
        _params: test_interface::BazParams,
        _results: test_interface::BazResults,
    ) -> Result<(), Error> {
        Err(Error::unimplemented("bar is not implemented".to_string()))
    }
}

#[derive(Default)]
pub struct TestStreamingImpl {
    i_sum: Cell<u32>,
    j_sum: Cell<u32>,
}

impl TestStreamingImpl {
    pub fn new() -> Self {
        Self::default()
    }
}

impl test_streaming::Server for TestStreamingImpl {
    async fn do_stream_i(
        self: Rc<Self>,
        params: test_streaming::DoStreamIParams,
    ) -> Result<(), Error> {
        let params = params.get()?;
        if params.get_throw_error() {
            return Err(Error::failed("throw requested".to_string()));
        }
        self.i_sum.set(self.i_sum.get() + params.get_i());
        Ok(())
    }

    async fn do_stream_j(
        self: Rc<Self>,
        params: test_streaming::DoStreamJParams,
    ) -> Result<(), Error> {
        let params = params.get()?;
        if params.get_throw_error() {
            return Err(Error::failed("throw requested".to_string()));
        }
        self.j_sum.set(self.j_sum.get() + params.get_j());
        Ok(())
    }

    async fn finish_stream(
        self: Rc<Self>,
        _params: test_streaming::FinishStreamParams,
        mut results: test_streaming::FinishStreamResults,
    ) -> Result<(), Error> {
        let mut results = results.get();
        results.set_total_i(self.i_sum.get());
        results.set_total_j(self.j_sum.get());
        Ok(())
    }
}

#[derive(Default)]
pub struct CssHandle {}

impl CssHandle {
    pub fn new() -> Self {
        Self::default()
    }
}

impl test_capability_server_set::handle::Server for CssHandle {}

#[derive(Default)]
pub struct TestCapabilityServerSet {
    set: Rc<
        RefCell<
            capnp_rpc::CapabilityServerSet<CssHandle, test_capability_server_set::handle::Client>,
        >,
    >,
}

impl TestCapabilityServerSet {
    pub fn new() -> Self {
        Self::default()
    }
}

impl test_capability_server_set::Server for TestCapabilityServerSet {
    async fn create_handle(
        self: Rc<Self>,
        _: test_capability_server_set::CreateHandleParams,
        mut results: test_capability_server_set::CreateHandleResults,
    ) -> Result<(), Error> {
        results
            .get()
            .set_handle(self.set.borrow_mut().new_client(CssHandle::new()));
        Ok(())
    }

    async fn check_handle(
        self: Rc<Self>,
        params: test_capability_server_set::CheckHandleParams,
        mut results: test_capability_server_set::CheckHandleResults,
    ) -> Result<(), Error> {
        let set = self.set.clone();
        let handle = params.get()?.get_handle()?;
        let resolved = capnp::capability::get_resolved_cap(handle).await;

        if set
            .borrow()
            .get_local_server_of_resolved(&resolved)
            .is_some()
        {
            results.get().set_is_ours(true)
        }

        Ok(())
    }
}

pub struct ResolverImpl {
    sender: RefCell<Option<oneshot::Sender<test_interface::Client>>>,
}

impl test_promise_resolve::resolver::Server for ResolverImpl {
    async fn resolve_to_another_promise(
        self: Rc<Self>,
        _params: test_promise_resolve::resolver::ResolveToAnotherPromiseParams,
        _results: test_promise_resolve::resolver::ResolveToAnotherPromiseResults,
    ) -> Result<(), Error> {
        let Some(sender) = self.sender.borrow_mut().take() else {
            return Err(Error::failed("no sender".into()));
        };

        let (snd, rcv) = oneshot::channel();
        let _ = sender.send(capnp_rpc::new_future_client(
            rcv.map_err(|_| Error::failed("oneshot was canceled".to_string())),
        ));
        *self.sender.borrow_mut() = Some(snd);
        Ok(())
    }

    async fn resolve_to_cap(
        self: Rc<Self>,
        _params: test_promise_resolve::resolver::ResolveToCapParams,
        _results: test_promise_resolve::resolver::ResolveToCapResults,
    ) -> Result<(), Error> {
        let Some(sender) = self.sender.borrow_mut().take() else {
            return Err(Error::failed("no sender".into()));
        };
        let _ = sender.send(capnp_rpc::new_client(TestInterface::new()));
        Ok(())
    }
}

pub struct TestPromiseResolveImpl {}

impl test_promise_resolve::Server for TestPromiseResolveImpl {
    async fn foo(
        self: Rc<Self>,
        _params: test_promise_resolve::FooParams,
        mut results: test_promise_resolve::FooResults,
    ) -> Result<(), Error> {
        let (snd, rcv) = oneshot::channel();
        let resolver = ResolverImpl {
            sender: RefCell::new(Some(snd)),
        };
        let mut results_root = results.get();
        results_root.set_cap(capnp_rpc::new_future_client(
            rcv.map_err(|_| Error::failed("oneshot was canceled".to_string())),
        ));
        results_root.set_resolver(capnp_rpc::new_client(resolver));
        Ok(())
    }
}

pub struct TestSelfImpl {
    foo_count: Cell<u32>,
}

impl TestSelfImpl {
    pub fn new() -> Self {
        Self {
            foo_count: Cell::new(0),
        }
    }
}

impl test_self::Server for TestSelfImpl {
    async fn foo(
        self: Rc<Self>,
        _params: test_self::FooParams,
        mut results: test_self::FooResults,
    ) -> Result<(), Error> {
        let foo_count = self.foo_count.get() + 1;
        self.foo_count.set(foo_count);
        results.get().set_x(foo_count);
        Ok(())
    }

    async fn get_self(
        self: Rc<Self>,
        _params: test_self::GetSelfParams,
        mut results: test_self::GetSelfResults,
    ) -> Result<(), Error> {
        results.get().set_cap(capnp_rpc::new_client_from_rc(self));
        Ok(())
    }
}
