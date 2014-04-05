/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use std::vec::Vec;

use capnp::capability::{FromServer, Server};
use capnp::list::{PrimitiveList};
use capnp::message::{MallocMessageBuilder, MessageBuilder};

use capnp_rpc::capability::{InitRequest, WaitForContent};
use capnp_rpc::ez_rpc::EzRpcServer;

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
    params : Option<PrimitiveList::Reader<f64>>) -> Result<f64, ~str> {

    match expression.which() {
        Some(Calculator::Expression::Literal(v)) => {
            Ok(v)
        },
        Some(Calculator::Expression::PreviousResult(p)) => {
            Ok(try!(p.read_request().send().wait()).get_value())
        }
        Some(Calculator::Expression::Parameter(p)) => {
            match params {
                None => {Err(~"bad parameter")}
                Some(params) => {
                    Ok(params[p as uint])
                }
            }
        }
        Some(Calculator::Expression::Call(call)) => {
            let func = call.get_function();
            let call_params = call.get_params();
            let mut param_values = Vec::new();
            for ii in range(0, call_params.size()) {
                let x = try!(evaluate_impl(call_params[ii], params));
                param_values.push(x);
            }
            let mut request = func.call_request();
            let request_params = request.init().init_params(param_values.len());
            for ii in range(0, param_values.len()) {
                request_params.set(ii, *param_values.get(ii));
            }
            Ok(try!(request.send().wait()).get_value())
        }
        None => fail!("unsupported expression"),
    }
}

struct FunctionImpl {
    param_count : uint,
    body : MallocMessageBuilder,
}

impl FunctionImpl {
    fn new(param_count : uint, body : Calculator::Expression::Reader) -> FunctionImpl {
        let mut result = FunctionImpl { param_count : param_count, body : MallocMessageBuilder::new_default() };
        result.body.set_root(&body);
        result
    }
}

impl Calculator::Function::Server for FunctionImpl {
    fn call(&mut self, mut context : Calculator::Function::CallContext) {
        let (params, results) = context.get();
        if params.get_params().size() != self.param_count{
            //"Wrong number of parameters."
            return context.fail();
        };

        {
            let expression = self.body.get_root::<Calculator::Expression::Builder>().as_reader();
            match evaluate_impl(expression, Some(params.get_params())) {
                Ok(r) => results.set_value(r),
                Err(_) => return context.fail(),
            }

        }
        context.done();
    }
}

pub struct OperatorImpl {
    op : Calculator::Operator::Reader,
}

impl Calculator::Function::Server for OperatorImpl {
    fn call(&mut self, mut context : Calculator::Function::CallContext) {
        let (params, results) = context.get();
        let params = params.get_params();
        if params.size() != 2 {
            //"Wrong number of parameters: {}", params.size()
            return context.fail();
        }

        let result = match self.op {
            Calculator::Operator::Add => params[0] + params[1],
            Calculator::Operator::Subtract => params[0] - params[1],
            Calculator::Operator::Multiply => params[0] * params[1],
            Calculator::Operator::Divide => params[0] / params[1],
        };

        results.set_value(result);
        context.done();
    }
}


struct CalculatorImpl;

impl Calculator::Server for CalculatorImpl {
    fn evaluate(&mut self, mut context : Calculator::EvaluateContext) {
        let (params, results) = context.get();
        match evaluate_impl(params.get_expression(), None) {
            Ok(r) => {
                results.set_value(
                    FromServer::new(
                        None::<EzRpcServer>,
                        ~ValueImpl::new(r)))
            }
            Err(_) => return context.fail(),
        }
        context.done();
    }
    fn def_function(&mut self, mut context : Calculator::DefFunctionContext) {
        let (params, results) = context.get();
        results.set_func(
            FromServer::new(
                None::<EzRpcServer>,
                ~FunctionImpl::new(params.get_param_count() as uint, params.get_body())));
        context.done();
    }
    fn get_operator(&mut self, mut context : Calculator::GetOperatorContext) {
        let (params, results) = context.get();
        results.set_func(
            match params.get_op() {
                Some(op) => {
                    FromServer::new(
                        None::<EzRpcServer>,
                        ~OperatorImpl {op : op})
                }
                None => fail!("Unknown operator."),
            });
        context.done();
    }
}

pub fn main() {
    let args = std::os::args();
    if args.len() != 3 {
        println!("usage: {} server ADDRESS[:PORT]", args[0]);
        return;
    }

    let rpc_server = EzRpcServer::new(args[2]).unwrap();

    // There's got to be a better way to do this.
    let calculator = (~Calculator::ServerDispatch { server : ~CalculatorImpl}) as ~Server:Send;
    rpc_server.export_cap("calculator", calculator);

    rpc_server.serve();
}
