/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;

use capnp::capability::{FromServer, ServerHook};
use capnp::list::{PrimitiveList};
use capnp::message::{MallocMessageBuilder, MessageBuilder};

use capnp_rpc::capability::{WaitForContent};

use calculator_capnp::Calculator;


struct ValueImpl {
    value : f64
}

impl ValueImpl {
    fn new(value : f64) -> ValueImpl {
        ValueImpl { value : value }
    }
}

impl Calculator::Value::Server for ValueImpl {
    fn read(&mut self, mut context : Calculator::Value::ReadContext) {
        let (_, results) = context.get();
        results.set_value(self.value);
        context.done();
    }
}

fn evaluate_impl(
    expression : Calculator::Expression::Reader,
    params : Option<PrimitiveList::Reader<f64>>) -> f64 {

    match expression.which() {
        Some(Calculator::Expression::Literal(v)) => v,
        Some(Calculator::Expression::PreviousResult(p)) => {
            p.read_request().send().wait().get_value()
        }
        Some(Calculator::Expression::Parameter(p)) => {
            match params {
                None => {fail!()}
                Some(params) => {
                    params[p as uint]
                }
            }
        }
        Some(Calculator::Expression::Call(call)) => {
            let _func = call.get_function();
            fail!()
        }
        None => fail!("unsupported expression"),
    }
}

struct FunctionImpl {
    param_count : uint,
    body : MallocMessageBuilder,
}

impl FunctionImpl {
    fn new(_param_count : uint, _body : Calculator::Expression::Reader) -> FunctionImpl {
        fail!()
    }
}

impl Calculator::Function::Server for FunctionImpl {
    fn call(&mut self, mut context : Calculator::Function::CallContext) {
        let (params, results) = context.get();
        assert!(params.get_params().size() == self.param_count,
                "Wrong number of parameters.");

        {
            let expression = self.body.get_root::<Calculator::Expression::Builder>().as_reader();
            results.set_value(evaluate_impl(expression, Some(params.get_params())));
        }
        context.done();
    }
}


struct CalculatorImpl<T>{
    server_hook : T,
}

impl <T : ServerHook> Calculator::Server for CalculatorImpl<T> {
    fn evaluate(&mut self, mut context : Calculator::EvaluateContext) {
        let (params, results) = context.get();
        results.set_value(
            FromServer::new(
                &self.server_hook,
                ~ValueImpl::new(evaluate_impl(params.get_expression(), None))));
        context.done();
    }
    fn def_function(&mut self, mut context : Calculator::DefFunctionContext) {
        let (params, results) = context.get();
        results.set_func(
            FromServer::new(
                &self.server_hook,
                ~FunctionImpl::new(params.get_param_count() as uint, params.get_body())));
    }
    fn get_operator(&mut self, mut context : Calculator::GetOperatorContext) {
        let (_params, _results) = context.get();
    }
}

pub fn main() {
    let args = std::os::args();
    if args.len() != 3 {
        println!("usage: {} server ADDRESS[:PORT]", args[0]);
        return;
    }

    println!("calculator server is unimplemented");
}
