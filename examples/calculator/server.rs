/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use std::vec::Vec;

use capnp::capability::{FromServer, Server};
use capnp::list::{primitive_list};
use capnp::{MallocMessageBuilder, MessageBuilder};

use capnp_rpc::capability::{InitRequest, LocalClient, WaitForContent};
use capnp_rpc::ez_rpc::EzRpcServer;

use calculator_capnp::calculator;


struct ValueImpl {
    value : f64
}

impl ValueImpl {
    fn new(value : f64) -> ValueImpl {
        ValueImpl { value : value }
    }
}

impl calculator::value::Server for ValueImpl {
    fn read(&mut self, mut context : calculator::value::ReadContext) {
        let (_, mut results) = context.get();
        results.set_value(self.value);
        context.done();
    }
}

fn evaluate_impl(
    expression : calculator::expression::Reader,
    params : Option<primitive_list::Reader<f64>>) -> Result<f64, String> {

    match expression.which() {
        Some(calculator::expression::Literal(v)) => {
            Ok(v)
        },
        Some(calculator::expression::PreviousResult(p)) => {
            Ok(try!(p.read_request().send().wait()).get_value())
        }
        Some(calculator::expression::Parameter(p)) => {
            match params {
                None => {Err("bad parameter".to_string())}
                Some(params) => {
                    Ok(params.get(p))
                }
            }
        }
        Some(calculator::expression::Call(call)) => {
            let func = call.get_function();
            let call_params = call.get_params();
            let mut param_values = Vec::new();
            for call_param in call_params.iter() {
                let x = try!(evaluate_impl(call_param, params));
                param_values.push(x);
            }
            let mut request = func.call_request();
            {
                let mut request_params = request.init().init_params(param_values.len() as u32);
                for ii in range(0, param_values.len()) {
                    request_params.set(ii as u32, param_values[ii]);
                }
            }
            Ok(try!(request.send().wait()).get_value())
        }
        None => panic!("unsupported expression"),
    }
}

struct FunctionImpl {
    param_count : u32,
    body : MallocMessageBuilder,
}

impl FunctionImpl {
    fn new(param_count : u32, body : calculator::expression::Reader) -> FunctionImpl {
        let mut result = FunctionImpl { param_count : param_count, body : MallocMessageBuilder::new_default() };
        result.body.set_root(body);
        result
    }
}

impl calculator::function::Server for FunctionImpl {
    fn call(&mut self, mut context : calculator::function::CallContext) {
        let (params, mut results) = context.get();
        if params.get_params().len() != self.param_count {
            //"Wrong number of parameters."
            return context.fail();
        };

        {
            match evaluate_impl(self.body.get_root::<calculator::expression::Builder>().as_reader(),
                                Some(params.get_params())) {
                Ok(r) => results.set_value(r),
                Err(_) => return context.fail(),
            }

        }
        context.done();
    }
}

#[derive(Copy)]
pub struct OperatorImpl {
    op : calculator::Operator,
}

impl calculator::function::Server for OperatorImpl {
    fn call(&mut self, mut context : calculator::function::CallContext) {
        let (params, mut results) = context.get();
        let params = params.get_params();
        if params.len() != 2 {
            //"Wrong number of parameters: {}", params.len()
            return context.fail();
        }

        let result = match self.op {
            calculator::Operator::Add => params.get(0) + params.get(1),
            calculator::Operator::Subtract => params.get(0) - params.get(1),
            calculator::Operator::Multiply => params.get(0) * params.get(1),
            calculator::Operator::Divide => params.get(0) / params.get(1),
        };

        results.set_value(result);
        context.done();
    }
}


struct CalculatorImpl;

impl calculator::Server for CalculatorImpl {
    fn evaluate(&mut self, mut context : calculator::EvaluateContext) {
        let (params, mut results) = context.get();
        match evaluate_impl(params.get_expression(), None) {
            Ok(r) => {
                results.set_value(
                    calculator::value::ToClient(ValueImpl::new(r)).from_server(None::<LocalClient>));
            }
            Err(_) => return context.fail(),
        }
        context.done();
    }
    fn def_function(&mut self, mut context : calculator::DefFunctionContext) {
        let (params, mut results) = context.get();
        results.set_func(
            calculator::function::ToClient(
                FunctionImpl::new(params.get_param_count() as u32, params.get_body()))
                .from_server(None::<LocalClient>));
        context.done();
    }
    fn get_operator<'a>(& mut self, mut context : calculator::GetOperatorContext<'a>) {
        {
            let (params, mut results) = context.get();
            results.set_func(
                match params.get_op() {
                    Some(op) => {
                        calculator::function::ToClient(OperatorImpl {op : op}).from_server(None::<LocalClient>)
                    }
                    None => panic!("Unknown operator."),
                });
        }
        context.done();
    }
}

pub fn main() {
    let args = std::os::args();
    if args.len() != 3 {
        println!("usage: {} server ADDRESS[:PORT]", args[0]);
        return;
    }

    let rpc_server = EzRpcServer::new(args[2].as_slice()).unwrap();

    // There's got to be a better way to do this.
    let calculator = Box::new(calculator::ServerDispatch { server : Box::new(CalculatorImpl)}) as Box<Server+Send>;
    rpc_server.export_cap("calculator", calculator);

    let _ = rpc_server.serve().join();
}
