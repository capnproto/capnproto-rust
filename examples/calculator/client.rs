/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use capnp_rpc::ez_rpc::EzRpcClient;
use capnp_rpc::rpc::{InitParams, WaitForContent};
use calculator_capnp::Calculator;

pub fn main() {
    let args = std::os::args();
    if args.len() != 3 {
        println!("usage: {} client <server address>", args[0]);
        return;
    }

    let mut rpc_client = EzRpcClient::new(args[2]);

    let calculator : Calculator::Client  = rpc_client.import_cap("calculator");

    {
        let mut req = calculator.evaluate_request();
        {
            let params = req.init_params();
            let exp = params.init_expression();
            exp.set_literal(123.45);
        }
        let mut res = req.send();
        let value = {
            let results = res.wait();
            results.get_value()
        };

        let mut result = value.read_request().send();
        println!("the value is: {}", result.wait().get_value());
    }


    {
        println!("Evaluating a literal... ");

        let mut request = calculator.evaluate_request();
        request.init_params().get_expression().set_literal(123.0);

        let eval_promise = request.send();

        let mut read_promise = eval_promise.pipeline.get_value().read_request().send();

        let response = read_promise.wait();
        assert_eq!(response.get_value(), 123.0);

        println!("PASS")
    }

    {
        println!("Using add and subtract... ");

        let add = {
            let mut request = calculator.get_operator_request();
            request.init_params().set_op(Calculator::Operator::Add);
            request.send().pipeline.get_func()
        };

        let subtract = {
            let mut request = calculator.get_operator_request();
            request.init_params().set_op(Calculator::Operator::Subtract);
            request.send().pipeline.get_func()
        };

        let mut request = calculator.evaluate_request();

        let subtract_call = request.init_params().get_expression().init_call();
        subtract_call.set_function(subtract);
        let subtract_params = subtract_call.init_params(2);
        subtract_params[1].set_literal(67.0);

        let add_call = subtract_params[0].init_call();
        add_call.set_function(add);
        let add_params = add_call.init_params(2);
        add_params[0].set_literal(123.0);
        add_params[1].set_literal(45.0);

        let eval_promise = request.send();
        let mut read_promise = eval_promise.pipeline.get_value().read_request().send();
        //let response = read_promise.wait();
        //assert_eq!(response.get_value(), 101.0);
    }


    rpc_client.netcat.wait();
}
