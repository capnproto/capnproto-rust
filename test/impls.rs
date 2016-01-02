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

use test_capnp::{bootstrap, test_interface, test_extends, test_pipeline,
                 test_call_order};
use gj::Promise;
use capnp::Error;
use capnp_rpc::rpc::LocalClient;

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
}

pub struct TestInterface {
    call_count: u64,
}

impl TestInterface {
    fn new() -> TestInterface {
        TestInterface { call_count: 0 }
    }
}

impl test_interface::Server for TestInterface {
    fn foo(&mut self,
           params: test_interface::FooParams,
           mut results: test_interface::FooResults)
           -> Promise<test_interface::FooResults, Error>
    {
        self.call_count += 1;
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
        Promise::err(Error::unimplemented("bar is not implemented".to_string()))
    }

    fn baz(&mut self,
           params: test_interface::BazParams,
           results: test_interface::BazResults)
           -> Promise<test_interface::BazResults, Error>
    {
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
               _params: test_pipeline::GetCapParams,
               results: test_pipeline::GetCapResults)
               -> Promise<test_pipeline::GetCapResults, Error>
    {
        Promise::ok(results)
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
