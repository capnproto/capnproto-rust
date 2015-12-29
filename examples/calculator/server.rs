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

use capnp::Error;
use capnp::primitive_list;

use capnp_rpc::rpc;
use capnp_rpc::rpc::LocalClient;
use capnp_rpc::twoparty;

use calculator_capnp::calculator;
use gj::{EventLoop, Promise, TaskReaper, TaskSet};
use gj::io::tcp;

struct ValueImpl {
    value: f64
}

impl ValueImpl {
    fn new(value: f64) -> ValueImpl {
        ValueImpl { value: value }
    }
}

impl calculator::value::Server for ValueImpl {
    fn read(&mut self,
            _params: calculator::value::ReadParams,
            mut results: calculator::value::ReadResults)
            -> Promise<calculator::value::ReadResults, Error>
    {
        results.get().set_value(self.value);
        Promise::ok(results)
    }
}

fn evaluate_impl(expression: calculator::expression::Reader,
                 params: Option<primitive_list::Reader<f64>>)
                 -> Promise<f64, Error>
{
    match pry!(expression.which()) {
        calculator::expression::Literal(v) => {
            Promise::ok(v)
        },
        calculator::expression::PreviousResult(p) => {
            pry!(p).read_request().send().promise.map(|v| {
                Ok(try!(v.get()).get_value())
            })
        }
        calculator::expression::Parameter(p) => {
            match params {
                Some(params) if p < params.len() => {
                    Promise::ok(params.get(p))
                }
                _ => {
                    Promise::err(Error::failed(format!("bad parameter: {}", p)))
                }
            }
        }
        calculator::expression::Call(call) => {
            let func = pry!(call.get_function());
            let mut param_promises = Vec::new();
            for param in pry!(call.get_params()).iter() {
                param_promises.push(evaluate_impl(param, params));
            }
            Promise::all(param_promises.into_iter()).then(move |param_values| {
                let mut request = func.call_request();
                {
                    let mut params = request.get().init_params(param_values.len() as u32);
                    for ii in 0..param_values.len() {
                        params.set(ii as u32, param_values[ii]);
                    }
                }
                request.send().promise.map(|result| {
                    Ok(try!(result.get()).get_value())
                })
            })
        }
    }
}

struct FunctionImpl {
    param_count: u32,
    body: ::capnp::message::Builder<::capnp::message::HeapAllocator>,
}

impl FunctionImpl {
    fn new(param_count: u32, body: calculator::expression::Reader) -> FunctionImpl {
        let mut result = FunctionImpl {
            param_count: param_count,
            body: ::capnp::message::Builder::new_default()
        };
        result.body.set_root(body).unwrap();
        result
    }
}

impl calculator::function::Server for FunctionImpl {
    fn call(&mut self,
            params: calculator::function::CallParams,
            mut results: calculator::function::CallResults)
        -> Promise<calculator::function::CallResults, Error>
    {
        let params = pry!(params.get().get_params());
        if params.len() != self.param_count {
            Promise::err(Error::failed(
                format!("Expect {} parameters but got {}.", self.param_count, params.len())))
        } else {
            evaluate_impl(
                pry!(self.body.get_root::<calculator::expression::Builder>()).as_reader(),
                Some(params)).map(move |v| {
                    results.get().set_value(v);
                    Ok(results)
                })
        }
    }
}

#[derive(Clone, Copy)]
pub struct OperatorImpl {
    op: calculator::Operator,
}

impl calculator::function::Server for OperatorImpl {
    fn call(&mut self,
            params: calculator::function::CallParams,
            mut results: calculator::function::CallResults)
            -> Promise<calculator::function::CallResults, Error>
    {
        let params = pry!(params.get().get_params());
        if params.len() != 2 {
            Promise::err(Error::failed("Wrong number of paramters.".to_string()))
        } else {
            let v = match self.op {
                calculator::Operator::Add =>       params.get(0) + params.get(1),
                calculator::Operator::Subtract =>  params.get(0) - params.get(1),
                calculator::Operator::Multiply =>  params.get(0) * params.get(1),
                calculator::Operator::Divide =>    params.get(0) / params.get(1),
            };
            results.get().set_value(v);
            Promise::ok(results)
        }
    }
}

struct CalculatorImpl;

impl calculator::Server for CalculatorImpl {
    fn evaluate(&mut self,
                params: calculator::EvaluateParams,
                mut results: calculator::EvaluateResults)
                -> Promise<calculator::EvaluateResults, Error>
    {
        evaluate_impl(pry!(params.get().get_expression()), None).map(move |v| {
            results.get().set_value(
                calculator::value::ToClient::new(ValueImpl::new(v)).from_server::<LocalClient>());
            Ok(results)
        })
    }
    fn def_function(&mut self,
                    params: calculator::DefFunctionParams,
                    mut results: calculator::DefFunctionResults)
                    -> Promise<calculator::DefFunctionResults, Error>
    {
        results.get().set_func(
            calculator::function::ToClient::new(
                FunctionImpl::new(params.get().get_param_count() as u32, pry!(params.get().get_body())))
                .from_server::<LocalClient>());
        Promise::ok(results)
    }
    fn get_operator(&mut self,
                    params: calculator::GetOperatorParams,
                    mut results: calculator::GetOperatorResults)
                    -> Promise<calculator::GetOperatorResults, Error>
    {
        let op = pry!(params.get().get_op());
        results.get().set_func(
            calculator::function::ToClient::new(OperatorImpl {op : op}).from_server::<LocalClient>());
        Promise::ok(results)
    }
}

pub fn accept_loop(listener: tcp::Listener,
                   mut task_set: TaskSet<(), Box<::std::error::Error>>,
                   calc: calculator::Client,
                   )
                   -> Promise<(), ::std::io::Error>
{
    listener.accept().lift().then(move |(listener, stream)| {
        let stream2 = stream.try_clone().unwrap();
        let mut network =
            twoparty::VatNetwork::new(stream, stream2, Default::default());
        let disconnect_promise = network.on_disconnect();

        // Should put in the calculator for the bootstrap.
        let rpc_system = rpc::System::new(Box::new(network), Some(calc.clone().client));

        task_set.add(disconnect_promise.attach(rpc_system).lift());
        accept_loop(listener, task_set, calc)
    })
}

struct Reaper;

impl TaskReaper<(), Box<::std::error::Error>> for Reaper {
    fn task_failed(&mut self, error: Box<::std::error::Error>) {
        println!("Task failed: {}", error);
    }
}

pub fn main() {
    let args : Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {} server ADDRESS[:PORT]", args[0]);
        return;
    }

    EventLoop::top_level(move |wait_scope| {
        use std::net::ToSocketAddrs;
        let addr = try!(args[2].to_socket_addrs()).next().expect("could not parse address");
        let listener = try!(tcp::Listener::bind(addr));


        let calc =
            calculator::ToClient::new(CalculatorImpl).from_server::<LocalClient>();

        let task_set = TaskSet::new(Box::new(Reaper));
        try!(accept_loop(listener, task_set, calc).wait(wait_scope));

        Ok(())
    }).expect("top level error");

    //let rpc_server = EzRpcServer::new(&*args[2]).unwrap();

    // There's got to be a better way to do this.
    //let calculator = Box::new(calculator::ServerDispatch { server : Box::new(CalculatorImpl)});

    //let _ = rpc_server.serve(calculator);
}
