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

use test_capnp::{bootstrap, test_interface, test_extends, test_extends2, test_pipeline,
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
                    _results: bootstrap::TestExtendsResults)
                    -> Promise<bootstrap::TestExtendsResults, Error>
    {
        unimplemented!()
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
                    _results: bootstrap::TestPipelineResults)
                    -> Promise<bootstrap::TestPipelineResults, Error>
    {
        unimplemented!()
    }

    fn test_call_order(&mut self,
                    _params: bootstrap::TestCallOrderParams,
                    _results: bootstrap::TestCallOrderResults)
                    -> Promise<bootstrap::TestCallOrderResults, Error>
    {
        unimplemented!()
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
        unimplemented!()
    }

    fn baz(&mut self,
           _params: test_interface::BazParams,
           _results: test_interface::BazResults)
           -> Promise<test_interface::BazResults, Error>
    {
        unimplemented!()
    }

}
