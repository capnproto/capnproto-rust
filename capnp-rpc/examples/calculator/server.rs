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

use capnp::primitive_list;
use capnp::Error;

use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};

use crate::calculator_capnp::calculator;
use capnp::capability::Promise;

use futures::future;
use futures::{AsyncReadExt, FutureExt, TryFutureExt};

struct ValueImpl {
    value: f64,
}

impl ValueImpl {
    fn new(value: f64) -> Self {
        Self { value }
    }
}

impl calculator::value::Server for ValueImpl {
    fn read(
        &mut self,
        _params: calculator::value::ReadParams,
        mut results: calculator::value::ReadResults,
    ) -> Promise<(), Error> {
        results.get().set_value(self.value);
        Promise::ok(())
    }
}

fn evaluate_impl(
    expression: calculator::expression::Reader,
    params: Option<primitive_list::Reader<f64>>,
) -> Promise<f64, Error> {
    match pry!(expression.which()) {
        calculator::expression::Literal(v) => Promise::ok(v),
        calculator::expression::PreviousResult(p) => Promise::from_future(
            pry!(p)
                .read_request()
                .send()
                .promise
                .map(|v| Ok(v?.get()?.get_value())),
        ),
        calculator::expression::Parameter(p) => match params {
            Some(params) if p < params.len() => Promise::ok(params.get(p)),
            _ => Promise::err(Error::failed(format!("bad parameter: {p}"))),
        },
        calculator::expression::Call(call) => {
            let func = pry!(call.get_function());
            let eval_params = future::try_join_all(
                pry!(call.get_params())
                    .iter()
                    .map(|p| evaluate_impl(p, params)),
            );
            Promise::from_future(async move {
                let param_values = eval_params.await?;
                let mut request = func.call_request();
                {
                    let mut params = request.get().init_params(param_values.len() as u32);
                    for (ii, value) in param_values.iter().enumerate() {
                        params.set(ii as u32, *value);
                    }
                }
                Ok(request.send().promise.await?.get()?.get_value())
            })
        }
    }
}

struct FunctionImpl {
    param_count: u32,
    body: ::capnp_rpc::ImbuedMessageBuilder<::capnp::message::HeapAllocator>,
}

impl FunctionImpl {
    fn new(param_count: u32, body: calculator::expression::Reader) -> ::capnp::Result<Self> {
        let mut result = Self {
            param_count,
            body: ::capnp_rpc::ImbuedMessageBuilder::new(::capnp::message::HeapAllocator::new()),
        };
        result.body.set_root(body)?;
        Ok(result)
    }
}

impl calculator::function::Server for FunctionImpl {
    fn call(
        &mut self,
        params: calculator::function::CallParams,
        mut results: calculator::function::CallResults,
    ) -> Promise<(), Error> {
        let params = pry!(pry!(params.get()).get_params());
        if params.len() != self.param_count {
            return Promise::err(Error::failed(format!(
                "Expected {} parameters but got {}.",
                self.param_count,
                params.len()
            )));
        }

        let eval = evaluate_impl(
            pry!(self.body.get_root::<calculator::expression::Builder>()).into_reader(),
            Some(params),
        );
        Promise::from_future(async move {
            results.get().set_value(eval.await?);
            Ok(())
        })
    }
}

#[derive(Clone, Copy)]
pub struct OperatorImpl {
    op: calculator::Operator,
}

impl calculator::function::Server for OperatorImpl {
    fn call(
        &mut self,
        params: calculator::function::CallParams,
        mut results: calculator::function::CallResults,
    ) -> Promise<(), Error> {
        let params = pry!(pry!(params.get()).get_params());
        if params.len() != 2 {
            Promise::err(Error::failed("Wrong number of paramters.".to_string()))
        } else {
            let v = match self.op {
                calculator::Operator::Add => params.get(0) + params.get(1),
                calculator::Operator::Subtract => params.get(0) - params.get(1),
                calculator::Operator::Multiply => params.get(0) * params.get(1),
                calculator::Operator::Divide => params.get(0) / params.get(1),
            };
            results.get().set_value(v);
            Promise::ok(())
        }
    }
}

struct CalculatorImpl;

impl calculator::Server for CalculatorImpl {
    fn evaluate(
        &mut self,
        params: calculator::EvaluateParams,
        mut results: calculator::EvaluateResults,
    ) -> Promise<(), Error> {
        Promise::from_future(async move {
            let v = evaluate_impl(params.get()?.get_expression()?, None).await?;
            results
                .get()
                .set_value(capnp_rpc::new_client(ValueImpl::new(v)));
            Ok(())
        })
    }
    fn def_function(
        &mut self,
        params: calculator::DefFunctionParams,
        mut results: calculator::DefFunctionResults,
    ) -> Promise<(), Error> {
        results
            .get()
            .set_func(capnp_rpc::new_client(pry!(FunctionImpl::new(
                pry!(params.get()).get_param_count() as u32,
                pry!(pry!(params.get()).get_body())
            ))));
        Promise::ok(())
    }
    fn get_operator(
        &mut self,
        params: calculator::GetOperatorParams,
        mut results: calculator::GetOperatorResults,
    ) -> Promise<(), Error> {
        let op = pry!(pry!(params.get()).get_op());
        results
            .get()
            .set_func(capnp_rpc::new_client(OperatorImpl { op }));
        Promise::ok(())
    }
}

pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::net::ToSocketAddrs;
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {} server ADDRESS[:PORT]", args[0]);
        return Ok(());
    }

    let addr = args[2]
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");

    tokio::task::LocalSet::new()
        .run_until(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            let calc: calculator::Client = capnp_rpc::new_client(CalculatorImpl);

            loop {
                let (stream, _) = listener.accept().await?;
                stream.set_nodelay(true)?;
                let (reader, writer) =
                    tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
                let network = twoparty::VatNetwork::new(
                    reader,
                    writer,
                    rpc_twoparty_capnp::Side::Server,
                    Default::default(),
                );

                let rpc_system = RpcSystem::new(Box::new(network), Some(calc.clone().client));
                tokio::task::spawn_local(rpc_system.map_err(|e| println!("error: {e:?}")));
            }
        })
        .await
}
