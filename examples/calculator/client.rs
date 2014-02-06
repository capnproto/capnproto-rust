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
        //# Make a request that just evaluates the literal value 123.
        //#
        //# What's interesting here is that evaluate() returns a "Value", which is
        //# another interface and therefore points back to an object living on the
        //# server.  We then have to call read() on that object to read it.
        //# However, even though we are making two RPC's, this block executes in
        //# *one* network round trip because of promise pipelining:  we do not wait
        //# for the first call to complete before we send the second call to the
        //# server.

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
        //# Make a request to evaluate 123 + 45 - 67.
        //#
        //# The Calculator interface requires that we first call getOperator() to
        //# get the addition and subtraction functions, then call evaluate() to use
        //# them.  But, once again, we can get both functions, call evaluate(), and
        //# then read() the result -- four RPCs -- in the time of *one* network
        //# round trip, because of promise pipelining.

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
        let response = read_promise.wait();
        assert_eq!(response.get_value(), 101.0);

        println!("PASS");
    }

    {
        //# Make a request to evaluate 4 * 6, then use the result in two more
        //# requests that add 3 and 5.
        //#
        //# Since evaluate() returns its result wrapped in a `Value`, we can pass
        //# that `Value` back to the server in subsequent requests before the first
        //# `evaluate()` has actually returned.  Thus, this example again does only
        //# one network round trip.

        println!("Pipelining eval() calls... ");

        let add = {
            let mut request = calculator.get_operator_request();
            request.init_params().set_op(Calculator::Operator::Add);
            request.send().pipeline.get_func()
        };

        let multiply = {
            let mut request = calculator.get_operator_request();
            request.init_params().set_op(Calculator::Operator::Multiply);
            request.send().pipeline.get_func()
        };

        //# Build the request to evaluate 4 * 6
        let mut request = calculator.evaluate_request();

        let multiply_call = request.init_params().get_expression().init_call();
        multiply_call.set_function(multiply);
        let multiply_params = multiply_call.init_params(2);
        multiply_params[0].set_literal(4.0);
        multiply_params[1].set_literal(6.0);

        let multiply_result = request.send().pipeline.get_value();

        //# Use the result in two calls that add 3 and 5.

        let mut add3_request = calculator.evaluate_request();
        let add3_call = add3_request.init_params().get_expression().init_call();
        add3_call.set_function(add.clone());
        let add3_params = add3_call.init_params(2);
        add3_params[0].set_previous_result(multiply_result.clone());
        add3_params[1].set_literal(3.0);
        let mut add3_promise = add3_request.send().pipeline.get_value().read_request().send();

        let mut add5_request = calculator.evaluate_request();
        let add5_call = add5_request.init_params().get_expression().init_call();
        add5_call.set_function(add);
        let add5_params = add5_call.init_params(2);
        add5_params[0].set_previous_result(multiply_result);
        add5_params[1].set_literal(5.0);
        let mut add5_promise = add5_request.send().pipeline.get_value().read_request().send();

        assert!(add3_promise.wait().get_value() == 27.0);
        assert!(add5_promise.wait().get_value() == 29.0);

        println!("PASS");
    }



    rpc_client.netcat.wait();
}
