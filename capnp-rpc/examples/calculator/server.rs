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

use std::cell::RefCell;

use ::capnp::message::HeapAllocator;
use capnp::primitive_list;
use capnp::Error;

use ::capnp_rpc::ImbuedMessageBuilder;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};

use crate::calculator_capnp::calculator;

use futures::future;
use futures::{AsyncReadExt, TryFutureExt};

struct ValueImpl {
    value: f64,
}

impl ValueImpl {
    fn new(value: f64) -> Self {
        Self { value }
    }
}

impl calculator::value::Server for ValueImpl {
    async fn read(
        self: std::rc::Rc<Self>,
        _params: calculator::value::ReadParams,
        mut results: calculator::value::ReadResults,
    ) -> Result<(), Error> {
        results.get().set_value(self.value);
        Ok(())
    }
}

async fn evaluate_impl(
    expression: calculator::expression::Reader<'_>,
    params: Option<primitive_list::Reader<'_, f64>>,
) -> Result<f64, Error> {
    match expression.which()? {
        calculator::expression::Literal(v) => Ok(v),
        calculator::expression::PreviousResult(p) => {
            let v = p?.read_request().send().promise.await?;
            Ok(v.get()?.get_value())
        }
        calculator::expression::Parameter(p) => match params {
            Some(params) if p < params.len() => Ok(params.get(p)),
            _ => Err(Error::failed(format!("bad parameter: {p}"))),
        },
        calculator::expression::Call(call) => {
            let func = call.get_function()?;
            let eval_params =
                future::try_join_all(call.get_params()?.iter().map(|p| evaluate_impl(p, params)));
            let param_values = eval_params.await?;
            let mut request = func.call_request();
            let mut params = request.get().init_params(param_values.len() as u32);
            for (ii, value) in param_values.iter().enumerate() {
                params.set(ii as u32, *value);
            }
            Ok(request.send().promise.await?.get()?.get_value())
        }
    }
}

struct FunctionImpl {
    param_count: u32,
    body: RefCell<ImbuedMessageBuilder<HeapAllocator>>,
}

impl FunctionImpl {
    fn new(param_count: u32, body: calculator::expression::Reader) -> ::capnp::Result<Self> {
        let result = Self {
            param_count,
            body: RefCell::new(ImbuedMessageBuilder::new(HeapAllocator::new())),
        };
        result.body.borrow_mut().set_root(body)?;
        Ok(result)
    }
}

impl calculator::function::Server for FunctionImpl {
    async fn call(
        self: std::rc::Rc<Self>,
        params: calculator::function::CallParams,
        mut results: calculator::function::CallResults,
    ) -> Result<(), Error> {
        let params = params.get()?.get_params()?;
        if params.len() != self.param_count {
            return Err(Error::failed(format!(
                "Expected {} parameters but got {}.",
                self.param_count,
                params.len()
            )));
        }

        let eval = evaluate_impl(
            self.body
                .borrow_mut()
                .get_root::<calculator::expression::Builder>()?
                .into_reader(),
            Some(params),
        )
        .await?;

        results.get().set_value(eval);
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct OperatorImpl {
    op: calculator::Operator,
}

impl calculator::function::Server for OperatorImpl {
    async fn call(
        self: std::rc::Rc<Self>,
        params: calculator::function::CallParams,
        mut results: calculator::function::CallResults,
    ) -> Result<(), Error> {
        let params = params.get()?.get_params()?;
        if params.len() != 2 {
            Err(Error::failed("Wrong number of parameters.".to_string()))
        } else {
            let v = match self.op {
                calculator::Operator::Add => params.get(0) + params.get(1),
                calculator::Operator::Subtract => params.get(0) - params.get(1),
                calculator::Operator::Multiply => params.get(0) * params.get(1),
                calculator::Operator::Divide => params.get(0) / params.get(1),
            };
            results.get().set_value(v);
            Ok(())
        }
    }
}

struct CalculatorImpl;

impl calculator::Server for CalculatorImpl {
    async fn evaluate(
        self: std::rc::Rc<Self>,
        params: calculator::EvaluateParams,
        mut results: calculator::EvaluateResults,
    ) -> Result<(), Error> {
        let v = evaluate_impl(params.get()?.get_expression()?, None).await?;
        results
            .get()
            .set_value(capnp_rpc::new_client(ValueImpl::new(v)));
        Ok(())
    }

    async fn def_function(
        self: std::rc::Rc<Self>,
        params: calculator::DefFunctionParams,
        mut results: calculator::DefFunctionResults,
    ) -> Result<(), Error> {
        results
            .get()
            .set_func(capnp_rpc::new_client(FunctionImpl::new(
                params.get()?.get_param_count() as u32,
                params.get()?.get_body()?,
            )?));
        Ok(())
    }

    async fn get_operator(
        self: std::rc::Rc<Self>,
        params: calculator::GetOperatorParams,
        mut results: calculator::GetOperatorResults,
    ) -> Result<(), Error> {
        let op = params.get()?.get_op()?;
        results
            .get()
            .set_func(capnp_rpc::new_client(OperatorImpl { op }));
        Ok(())
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
                    futures::io::BufReader::new(reader),
                    futures::io::BufWriter::new(writer),
                    rpc_twoparty_capnp::Side::Server,
                    Default::default(),
                );

                let rpc_system = RpcSystem::new(Box::new(network), Some(calc.clone().client));
                tokio::task::spawn_local(rpc_system.map_err(|e| println!("error: {e:?}")));
            }
        })
        .await
}
