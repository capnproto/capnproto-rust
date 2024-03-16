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
    test_interface, test_more_stuff, test_pipeline,
};

use capnp::capability::Promise;
use capnp::Error;
use capnp_rpc::pry;

use futures::{FutureExt, TryFutureExt};

use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Bootstrap;

impl bootstrap::Server for Bootstrap {
    fn test_interface(
        &mut self,
        _params: bootstrap::TestInterfaceParams,
        mut results: bootstrap::TestInterfaceResults,
    ) -> Result<Promise<(), Error>, Error> {
        {
            results
                .get()
                .set_cap(capnp_rpc::new_client(TestInterface::new()));
        }
        Ok(Promise::ok(()))
    }

    fn test_extends(
        &mut self,
        _params: bootstrap::TestExtendsParams,
        mut results: bootstrap::TestExtendsResults,
    ) -> Result<Promise<(), Error>, Error> {
        {
            results.get().set_cap(capnp_rpc::new_client(TestExtends));
        }
        Ok(Promise::ok(()))
    }

    fn test_extends2(
        &mut self,
        _params: bootstrap::TestExtends2Params,
        _results: bootstrap::TestExtends2Results,
    ) -> Result<Promise<(), Error>, Error> {
        unimplemented!()
    }

    fn test_pipeline(
        &mut self,
        _params: bootstrap::TestPipelineParams,
        mut results: bootstrap::TestPipelineResults,
    ) -> Result<Promise<(), Error>, Error> {
        {
            results.get().set_cap(capnp_rpc::new_client(TestPipeline));
        }
        Ok(Promise::ok(()))
    }

    fn test_call_order(
        &mut self,
        _params: bootstrap::TestCallOrderParams,
        mut results: bootstrap::TestCallOrderResults,
    ) -> Result<Promise<(), Error>, Error> {
        {
            results
                .get()
                .set_cap(capnp_rpc::new_client(TestCallOrder::new()));
        }
        Ok(Promise::ok(()))
    }
    fn test_more_stuff(
        &mut self,
        _params: bootstrap::TestMoreStuffParams,
        mut results: bootstrap::TestMoreStuffResults,
    ) -> Result<Promise<(), Error>, Error> {
        {
            results
                .get()
                .set_cap(capnp_rpc::new_client(TestMoreStuff::new()));
        }
        Ok(Promise::ok(()))
    }
    fn test_capability_server_set(
        &mut self,
        _params: bootstrap::TestCapabilityServerSetParams,
        mut results: bootstrap::TestCapabilityServerSetResults,
    ) -> Result<Promise<(), Error>, Error> {
        results
            .get()
            .set_cap(capnp_rpc::new_client(TestCapabilityServerSet::new()));
        Ok(Promise::ok(()))
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
    fn foo(
        &mut self,
        params: test_interface::FooParams,
        mut results: test_interface::FooResults,
    ) -> Result<Promise<(), Error>, Error> {
        self.increment_call_count();
        let params = params.get()?;
        if params.get_i() != 123 {
            return Err(Error::failed("expected i to equal 123".to_string()));
        }
        if !params.get_j() {
            return Err(Error::failed("expected j to be true".to_string()));
        }
        {
            let mut results = results.get();
            results.set_x("foo".into());
        }
        Ok(Promise::ok(()))
    }

    fn bar(
        &mut self,
        _params: test_interface::BarParams,
        _results: test_interface::BarResults,
    ) -> Result<Promise<(), Error>, Error> {
        self.increment_call_count();
        Err(Error::unimplemented("bar is not implemented".to_string()))
    }

    fn baz(
        &mut self,
        params: test_interface::BazParams,
        _results: test_interface::BazResults,
    ) -> Result<Promise<(), Error>, Error> {
        self.increment_call_count();
        crate::test_util::CheckTestMessage::check_test_message(params.get()?.get_s()?);
        Ok(Promise::ok(()))
    }
}

struct TestExtends;

impl test_interface::Server for TestExtends {
    fn foo(
        &mut self,
        params: test_interface::FooParams,
        mut results: test_interface::FooResults,
    ) -> Result<Promise<(), Error>, Error> {
        let params = params.get()?;
        if params.get_i() != 321 {
            return Err(Error::failed("expected i to equal 321".to_string()));
        }
        if params.get_j() {
            return Err(Error::failed("expected j to be false".to_string()));
        }
        {
            let mut results = results.get();
            results.set_x("bar".into());
        }
        Ok(Promise::ok(()))
    }

    fn bar(
        &mut self,
        _params: test_interface::BarParams,
        _results: test_interface::BarResults,
    ) -> Result<Promise<(), Error>, Error> {
        Err(Error::unimplemented("bar is not implemented".to_string()))
    }

    fn baz(
        &mut self,
        _params: test_interface::BazParams,
        _results: test_interface::BazResults,
    ) -> Result<Promise<(), Error>, Error> {
        Err(Error::unimplemented("baz is not implemented".to_string()))
    }
}

impl test_extends::Server for TestExtends {
    fn qux(
        &mut self,
        _params: test_extends::QuxParams,
        _results: test_extends::QuxResults,
    ) -> Result<Promise<(), Error>, Error> {
        Err(Error::unimplemented("qux is not implemented".to_string()))
    }

    fn corge(
        &mut self,
        _params: test_extends::CorgeParams,
        _results: test_extends::CorgeResults,
    ) -> Result<Promise<(), Error>, Error> {
        Err(Error::unimplemented("corge is not implemented".to_string()))
    }

    fn grault(
        &mut self,
        _params: test_extends::GraultParams,
        mut results: test_extends::GraultResults,
    ) -> Result<Promise<(), Error>, Error> {
        crate::test_util::init_test_message(results.get());
        Ok(Promise::ok(()))
    }
}

struct TestPipeline;

impl test_pipeline::Server for TestPipeline {
    fn get_cap(
        &mut self,
        params: test_pipeline::GetCapParams,
        mut results: test_pipeline::GetCapResults,
    ) -> Result<Promise<(), Error>, Error> {
        if params.get()?.get_n() != 234 {
            return Err(Error::failed("expected n to equal 234".to_string()));
        }
        let cap = params.get()?.get_in_cap()?;
        let mut request = cap.foo_request();
        request.get().set_i(123);
        request.get().set_j(true);
        Ok(Promise::from_future(request.send().promise.map(
            move |response| {
                if response?.get()?.get_x()? != "foo" {
                    return Err(Error::failed("expected x to equal 'foo'".to_string()));
                }

                results.get().set_s("bar".into());

                // TODO implement better casting
                results
                    .get()
                    .init_out_box()
                    .set_cap(test_interface::Client {
                        client: capnp_rpc::new_client::<test_extends::Client, _>(TestExtends)
                            .client,
                    });
                Ok(())
            },
        )))
    }

    fn get_null_cap(
        &mut self,
        _params: test_pipeline::GetNullCapParams,
        _results: test_pipeline::GetNullCapResults,
    ) -> Result<Promise<(), Error>, Error> {
        Ok(Promise::ok(()))
    }
}

#[derive(Default)]
pub struct TestCallOrder {
    count: u32,
}

impl TestCallOrder {
    pub fn new() -> Self {
        Self::default()
    }
}

impl test_call_order::Server for TestCallOrder {
    fn get_call_sequence(
        &mut self,
        _params: test_call_order::GetCallSequenceParams,
        mut results: test_call_order::GetCallSequenceResults,
    ) -> Result<Promise<(), Error>, Error> {
        results.get().set_n(self.count);
        self.count += 1;
        Ok(Promise::ok(()))
    }
}

#[derive(Default)]
pub struct TestMoreStuff {
    call_count: u32,
    handle_count: Rc<Cell<i64>>,
    client_to_hold: Option<test_interface::Client>,
}

impl TestMoreStuff {
    pub fn new() -> Self {
        Self::default()
    }
    /*
    pub fn get_call_count(&self) -> Rc<Cell<u64>> {
        self.call_count.clone()
    }
    fn increment_call_count(&self) {
        self.call_count.set(self.call_count.get() + 1);
    } */
}

impl test_call_order::Server for TestMoreStuff {
    fn get_call_sequence(
        &mut self,
        _params: test_call_order::GetCallSequenceParams,
        mut results: test_call_order::GetCallSequenceResults,
    ) -> Result<Promise<(), Error>, Error> {
        results.get().set_n(self.call_count);
        self.call_count += 1;
        Ok(Promise::ok(()))
    }
}

impl test_more_stuff::Server for TestMoreStuff {
    fn call_foo(
        &mut self,
        params: test_more_stuff::CallFooParams,
        mut results: test_more_stuff::CallFooResults,
    ) -> Result<Promise<(), Error>, Error> {
        self.call_count += 1;
        let cap = params.get()?.get_cap()?;
        let mut request = cap.foo_request();
        request.get().set_i(123);
        request.get().set_j(true);

        Ok(Promise::from_future(request.send().promise.map(
            move |response| {
                if response?.get()?.get_x()? != "foo" {
                    return Err(Error::failed("expected x to equal 'foo'".to_string()));
                }
                results.get().set_s("bar".into());
                Ok(())
            },
        )))
    }

    fn call_foo_when_resolved(
        &mut self,
        params: test_more_stuff::CallFooWhenResolvedParams,
        mut results: test_more_stuff::CallFooWhenResolvedResults,
    ) -> Result<Promise<(), Error>, Error> {
        self.call_count += 1;
        let cap = params.get()?.get_cap()?;
        Ok(Promise::from_future(cap.client.when_resolved().and_then(
            move |()| {
                let mut request = cap.foo_request();
                request.get().set_i(123);
                request.get().set_j(true);
                request.send().promise.map(move |response| {
                    if response?.get()?.get_x()? != "foo" {
                        return Err(Error::failed("expected x to equal 'foo'".to_string()));
                    }
                    results.get().set_s("bar".into());
                    Ok(())
                })
            },
        )))
    }

    fn never_return(
        &mut self,
        params: test_more_stuff::NeverReturnParams,
        mut results: test_more_stuff::NeverReturnResults,
    ) -> Result<Promise<(), Error>, Error> {
        self.call_count += 1;

        let cap = params.get()?.get_cap()?;

        // Attach `cap` to the promise to make sure it is released.
        let attached = cap.clone();
        let promise = Promise::from_future(::futures::future::pending().map_ok(|()| {
            drop(attached);
        }));

        // Also attach `cap` to the result struct so we can make sure that the results are released.
        results.get().set_cap_copy(cap);

        Ok(promise)
    }

    fn hold(
        &mut self,
        params: test_more_stuff::HoldParams,
        _results: test_more_stuff::HoldResults,
    ) -> Result<Promise<(), Error>, Error> {
        self.call_count += 1;
        self.client_to_hold = Some(params.get()?.get_cap()?);
        Ok(Promise::ok(()))
    }

    fn dont_hold(
        &mut self,
        params: test_more_stuff::DontHoldParams,
        _results: test_more_stuff::DontHoldResults,
    ) -> Result<Promise<(), Error>, Error> {
        self.call_count += 1;
        let _ = Some(params.get()?.get_cap()?);
        Ok(Promise::ok(()))
    }

    fn call_held(
        &mut self,
        _params: test_more_stuff::CallHeldParams,
        mut results: test_more_stuff::CallHeldResults,
    ) -> Result<Promise<(), Error>, Error> {
        self.call_count += 1;
        match self.client_to_hold {
            None => Err(Error::failed("no held client".to_string())),
            Some(ref client) => {
                let mut request = client.foo_request();
                {
                    let mut params = request.get();
                    params.set_i(123);
                    params.set_j(true);
                }
                Ok(Promise::from_future(request.send().promise.map(
                    move |response| {
                        if response?.get()?.get_x()? != "foo" {
                            Err(Error::failed("expected X to equal 'foo'".to_string()))
                        } else {
                            results.get().set_s("bar".into());
                            Ok(())
                        }
                    },
                )))
            }
        }
    }

    fn get_held(
        &mut self,
        _params: test_more_stuff::GetHeldParams,
        mut results: test_more_stuff::GetHeldResults,
    ) -> Result<Promise<(), Error>, Error> {
        self.call_count += 1;
        match self.client_to_hold {
            None => Err(Error::failed("no held client".to_string())),
            Some(ref client) => {
                results.get().set_cap(client.clone());
                Ok(Promise::ok(()))
            }
        }
    }

    fn echo(
        &mut self,
        params: test_more_stuff::EchoParams,
        mut results: test_more_stuff::EchoResults,
    ) -> Result<Promise<(), Error>, Error> {
        self.call_count += 1;
        results.get().set_cap(params.get()?.get_cap()?);
        Ok(Promise::ok(()))
    }

    fn expect_cancel(
        &mut self,
        _params: test_more_stuff::ExpectCancelParams,
        _results: test_more_stuff::ExpectCancelResults,
    ) -> Result<Promise<(), Error>, Error> {
        unimplemented!()
    }

    fn get_handle(
        &mut self,
        _params: test_more_stuff::GetHandleParams,
        mut results: test_more_stuff::GetHandleResults,
    ) -> Result<Promise<(), Error>, Error> {
        self.call_count += 1;
        let handle = Handle::new(&self.handle_count);
        results.get().set_handle(capnp_rpc::new_client(handle));
        Ok(Promise::ok(()))
    }

    fn get_handle_count(
        &mut self,
        _params: test_more_stuff::GetHandleCountParams,
        mut results: test_more_stuff::GetHandleCountResults,
    ) -> Result<Promise<(), Error>, Error> {
        self.call_count += 1;
        results.get().set_count(self.handle_count.get());
        Ok(Promise::ok(()))
    }

    fn get_null(
        &mut self,
        _params: test_more_stuff::GetNullParams,
        _results: test_more_stuff::GetNullResults,
    ) -> Result<Promise<(), Error>, Error> {
        unimplemented!()
    }

    fn method_with_defaults(
        &mut self,
        _params: test_more_stuff::MethodWithDefaultsParams,
        _results: test_more_stuff::MethodWithDefaultsResults,
    ) -> Result<Promise<(), Error>, Error> {
        unimplemented!()
    }

    fn call_each_capability(
        &mut self,
        params: test_more_stuff::CallEachCapabilityParams,
        _results: test_more_stuff::CallEachCapabilityResults,
    ) -> Result<Promise<(), Error>, Error> {
        let mut results = Vec::new();
        for cap in params.get()?.get_caps()? {
            let mut request = cap?.foo_request();
            request.get().set_i(123);
            request.get().set_j(true);
            results.push(request.send().promise);
        }

        Ok(Promise::from_future(
            ::futures::future::try_join_all(results).map_ok(|_| ()),
        ))
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
    fulfiller: Option<::futures::channel::oneshot::Sender<()>>,
    imp: TestInterface,
}

impl TestCapDestructor {
    pub fn new(fulfiller: ::futures::channel::oneshot::Sender<()>) -> Self {
        Self {
            fulfiller: Some(fulfiller),
            imp: TestInterface::new(),
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
    fn foo(
        &mut self,
        params: test_interface::FooParams,
        results: test_interface::FooResults,
    ) -> Result<Promise<(), Error>, Error> {
        Ok(self.imp.foo(params, results)?)
    }

    fn bar(
        &mut self,
        _params: test_interface::BarParams,
        _results: test_interface::BarResults,
    ) -> Result<Promise<(), Error>, Error> {
        Err(Error::unimplemented("bar is not implemented".to_string()))
    }

    fn baz(
        &mut self,
        _params: test_interface::BazParams,
        _results: test_interface::BazResults,
    ) -> Result<Promise<(), Error>, Error> {
        Err(Error::unimplemented("bar is not implemented".to_string()))
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
    fn create_handle(
        &mut self,
        _: test_capability_server_set::CreateHandleParams,
        mut results: test_capability_server_set::CreateHandleResults,
    ) -> Result<Promise<(), Error>, Error> {
        results
            .get()
            .set_handle(self.set.borrow_mut().new_client(CssHandle::new()));
        Ok(Promise::ok(()))
    }

    fn check_handle(
        &mut self,
        params: test_capability_server_set::CheckHandleParams,
        mut results: test_capability_server_set::CheckHandleResults,
    ) -> Result<Promise<(), Error>, Error> {
        let set = self.set.clone();
        let handle = params.get()?.get_handle()?;
        Ok(Promise::from_future(async move {
            let resolved = capnp::capability::get_resolved_cap(handle).await;
            match set.borrow().get_local_server_of_resolved(&resolved) {
                None => (),
                Some(_) => results.get().set_is_ours(true),
            }
            Ok(())
        }))
    }
}
