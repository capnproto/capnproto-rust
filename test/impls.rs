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

use test_capnp::{bootstrap, test_handle, test_interface, test_extends, test_pipeline,
                 test_call_order, test_more_stuff};
use gj::Promise;
use capnp::Error;
use capnp_rpc::rpc::LocalClient;

use std::cell::Cell;
use std::rc::Rc;

pub struct Bootstrap;

impl bootstrap::Server for Bootstrap {
    fn test_interface(&mut self,
                      _params: bootstrap::TestInterfaceParams,
                      mut results: bootstrap::TestInterfaceResults)
                      -> Promise<bootstrap::TestInterfaceResults, Error>
    {
        {
            results.get().set_cap(
                test_interface::ToClient::new(TestInterface::new()).from_server::<LocalClient>());
        }
        Promise::ok(results)
    }

    fn test_extends(&mut self,
                    _params: bootstrap::TestExtendsParams,
                    mut results: bootstrap::TestExtendsResults)
                    -> Promise<bootstrap::TestExtendsResults, Error>
    {
        {
            results.get().set_cap(
                test_extends::ToClient::new(TestExtends).from_server::<LocalClient>());
        }
        Promise::ok(results)
    }

    fn test_extends2(&mut self,
                    _params: bootstrap::TestExtends2Params,
                    _results: bootstrap::TestExtends2Results)
                    -> Promise<bootstrap::TestExtends2Results, Error>
    {
        unimplemented!()
    }

    fn test_pipeline(&mut self,
                    _params: bootstrap::TestPipelineParams,
                    mut results: bootstrap::TestPipelineResults)
                    -> Promise<bootstrap::TestPipelineResults, Error>
    {
        {
            results.get().set_cap(
                test_pipeline::ToClient::new(TestPipeline).from_server::<LocalClient>());
        }
        Promise::ok(results)
    }

    fn test_call_order(&mut self,
                    _params: bootstrap::TestCallOrderParams,
                    mut results: bootstrap::TestCallOrderResults)
                    -> Promise<bootstrap::TestCallOrderResults, Error>
    {
        {
            results.get().set_cap(
                test_call_order::ToClient::new(TestCallOrder::new()).from_server::<LocalClient>());
        }
        Promise::ok(results)
    }
    fn test_more_stuff(&mut self,
                       _params: bootstrap::TestMoreStuffParams,
                       mut results: bootstrap::TestMoreStuffResults)
                       -> Promise<bootstrap::TestMoreStuffResults, Error>
    {
        {
            results.get().set_cap(
                test_more_stuff::ToClient::new(TestMoreStuff::new()).from_server::<LocalClient>());
        }
        Promise::ok(results)
    }
}

pub struct TestInterface {
    call_count: Rc<Cell<u64>>,
}

impl TestInterface {
    pub fn new() -> TestInterface {
        TestInterface { call_count: Rc::new(Cell::new(0)) }
    }
    pub fn get_call_count(&self) -> Rc<Cell<u64>> {
        self.call_count.clone()
    }
    fn increment_call_count(&self) {
        self.call_count.set(self.call_count.get() + 1);
    }
}

impl test_interface::Server for TestInterface {
    fn foo(&mut self,
           params: test_interface::FooParams,
           mut results: test_interface::FooResults)
           -> Promise<test_interface::FooResults, Error>
    {
        self.increment_call_count();
        let params = params.get();
        if params.get_i() != 123 {
            return Promise::err(Error::failed(format!("expected i to equal 123")));
        }
        if !params.get_j() {
            return Promise::err(Error::failed(format!("expected j to be true")));
        }
        {
            let mut results = results.get();
            results.set_x("foo");
        }
          Promise::ok(results)
    }

    fn bar(&mut self,
           _params: test_interface::BarParams,
           _results: test_interface::BarResults)
           -> Promise<test_interface::BarResults, Error>
    {
        self.increment_call_count();
        Promise::err(Error::unimplemented("bar is not implemented".to_string()))
    }

    fn baz(&mut self,
           params: test_interface::BazParams,
           results: test_interface::BazResults)
           -> Promise<test_interface::BazResults, Error>
    {
        self.increment_call_count();
        ::test_util::CheckTestMessage::check_test_message(pry!(params.get().get_s()));
        Promise::ok(results)
    }

}

struct TestExtends;

impl test_interface::Server for TestExtends {
   fn foo(&mut self,
           params: test_interface::FooParams,
           mut results: test_interface::FooResults)
           -> Promise<test_interface::FooResults, Error>
    {
        let params = params.get();
        if params.get_i() != 321 {
            return Promise::err(Error::failed(format!("expected i to equal 321")));
        }
        if params.get_j() {
            return Promise::err(Error::failed(format!("expected j to be false")));
        }
        {
            let mut results = results.get();
            results.set_x("bar");
        }
        Promise::ok(results)
    }

    fn bar(&mut self,
           _params: test_interface::BarParams,
           _results: test_interface::BarResults)
           -> Promise<test_interface::BarResults, Error>
    {
        Promise::err(Error::unimplemented("bar is not implemented".to_string()))
    }

    fn baz(&mut self,
           _params: test_interface::BazParams,
           _results: test_interface::BazResults)
           -> Promise<test_interface::BazResults, Error>
    {
        Promise::err(Error::unimplemented("baz is not implemented".to_string()))
    }
}

impl test_extends::Server for TestExtends {
  fn qux(&mut self,
           _params: test_extends::QuxParams,
           _results: test_extends::QuxResults)
           -> Promise<test_extends::QuxResults, Error>
    {
        Promise::err(Error::unimplemented("qux is not implemented".to_string()))
    }

  fn corge(&mut self,
           _params: test_extends::CorgeParams,
           _results: test_extends::CorgeResults)
           -> Promise<test_extends::CorgeResults, Error>
    {
        Promise::err(Error::unimplemented("corge is not implemented".to_string()))
    }

  fn grault(&mut self,
           _params: test_extends::GraultParams,
           mut results: test_extends::GraultResults)
           -> Promise<test_extends::GraultResults, Error>
    {
        ::test_util::init_test_message(results.get());
        Promise::ok(results)
    }
}

struct TestPipeline;

impl test_pipeline::Server for TestPipeline {
    fn get_cap(&mut self,
               params: test_pipeline::GetCapParams,
               mut results: test_pipeline::GetCapResults)
               -> Promise<test_pipeline::GetCapResults, Error>
    {
        if params.get().get_n() != 234 {
            return Promise::err(Error::failed("expected n to equal 234".to_string()));
        }
        let cap = pry!(params.get().get_in_cap());
        let mut request = cap.foo_request();
        request.get().set_i(123);
        request.get().set_j(true);
        request.send().promise.map(move |response| {
            if try!(try!(response.get()).get_x()) != "foo" {
                return Err(Error::failed("expected x to equal 'foo'".to_string()));
            }

            results.get().set_s("bar");

            // TODO implement better casting
            results.get().init_out_box().set_cap(
                test_interface::Client {
                    client:
                    test_extends::ToClient::new(TestExtends).from_server::<LocalClient>().client,
                });
            Ok(results)
        })
    }
}

struct TestCallOrder {
    count: u32,
}

impl TestCallOrder {
    fn new() -> TestCallOrder {
        TestCallOrder { count: 0 }
    }
}

impl test_call_order::Server for TestCallOrder {
    fn get_call_sequence(&mut self,
                         _params: test_call_order::GetCallSequenceParams,
                         mut results: test_call_order::GetCallSequenceResults)
                         -> Promise<test_call_order::GetCallSequenceResults, Error>
    {
        results.get().set_n(self.count);
        self.count += 1;
        Promise::ok(results)
    }
}

struct TestMoreStuff {
    _call_count: Rc<Cell<i64>>,
    handle_count: Rc<Cell<i64>>,
}

impl TestMoreStuff {
    pub fn new() -> TestMoreStuff {
        TestMoreStuff {
            _call_count: Rc::new(Cell::new(0)),
            handle_count: Rc::new(Cell::new(0)),
        }
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
    fn get_call_sequence(&mut self,
                         _params: test_call_order::GetCallSequenceParams,
                         results: test_call_order::GetCallSequenceResults)
                         -> Promise<test_call_order::GetCallSequenceResults, Error>
    {
        Promise::ok(results)
    }
}

impl test_more_stuff::Server for TestMoreStuff {
    fn call_foo(&mut self,
                _params: test_more_stuff::CallFooParams,
                _results: test_more_stuff::CallFooResults)
                -> Promise<test_more_stuff::CallFooResults, Error>
    {
        unimplemented!()
    }

    fn call_foo_when_resolved(&mut self,
                              _params: test_more_stuff::CallFooWhenResolvedParams,
                              _results: test_more_stuff::CallFooWhenResolvedResults)
                              -> Promise<test_more_stuff::CallFooWhenResolvedResults, Error>
    {
        unimplemented!()
    }

    fn never_return(&mut self,
                    _params: test_more_stuff::NeverReturnParams,
                    _results: test_more_stuff::NeverReturnResults)
                    -> Promise<test_more_stuff::NeverReturnResults, Error>
    {
        unimplemented!()
    }

    fn hold(&mut self,
            _params: test_more_stuff::HoldParams,
            _results: test_more_stuff::HoldResults)
            -> Promise<test_more_stuff::HoldResults, Error>
    {
        unimplemented!()
    }

    fn call_held(&mut self,
                 _params: test_more_stuff::CallHeldParams,
                 _results: test_more_stuff::CallHeldResults)
                 -> Promise<test_more_stuff::CallHeldResults, Error>
    {
        unimplemented!()
    }

    fn get_held(&mut self,
                _params: test_more_stuff::GetHeldParams,
                _results: test_more_stuff::GetHeldResults)
                -> Promise<test_more_stuff::GetHeldResults, Error>
    {
        unimplemented!()
    }

    fn echo(&mut self,
            _params: test_more_stuff::EchoParams,
            _results: test_more_stuff::EchoResults)
            -> Promise<test_more_stuff::EchoResults, Error>
    {
        unimplemented!()
    }

    fn expect_cancel(&mut self,
                     _params: test_more_stuff::ExpectCancelParams,
                     _results: test_more_stuff::ExpectCancelResults)
                     -> Promise<test_more_stuff::ExpectCancelResults, Error>
    {
        unimplemented!()
    }

    fn get_handle(&mut self,
                  _params: test_more_stuff::GetHandleParams,
                  mut results: test_more_stuff::GetHandleResults)
                  -> Promise<test_more_stuff::GetHandleResults, Error>
    {
        let handle = Handle::new(&self.handle_count);
        results.get().set_handle(
            test_handle::ToClient::new(handle).from_server::<LocalClient>());
        Promise::ok(results)
    }

    fn get_handle_count(&mut self,
                        _params: test_more_stuff::GetHandleCountParams,
                        mut results: test_more_stuff::GetHandleCountResults)
                        -> Promise<test_more_stuff::GetHandleCountResults, Error>
    {
        results.get().set_count(self.handle_count.get());
        Promise::ok(results)
    }

    fn get_null(&mut self,
                _params: test_more_stuff::GetNullParams,
                _results: test_more_stuff::GetNullResults)
                -> Promise<test_more_stuff::GetNullResults, Error>
    {
        unimplemented!()
    }

    fn method_with_defaults(&mut self,
                            _params: test_more_stuff::MethodWithDefaultsParams,
                            _results: test_more_stuff::MethodWithDefaultsResults)
                            -> Promise<test_more_stuff::MethodWithDefaultsResults, Error>
    {
        unimplemented!()
    }

}

struct Handle {
    count: Rc<Cell<i64>>,
}

impl Handle {
    fn new(count: &Rc<Cell<i64>>) -> Handle {
        let count = count.clone();
        count.set(count.get() + 1);
        Handle { count: count }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.count.set(self.count.get() - 1);
    }
}

impl test_handle::Server for Handle {}
