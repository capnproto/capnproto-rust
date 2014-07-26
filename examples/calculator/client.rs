/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use capnp::capability::{FromServer};
use capnp_rpc::ez_rpc::EzRpcClient;
use capnp_rpc::capability::{InitRequest, LocalClient, WaitForContent};
use calculator_capnp::Calculator;

pub struct PowerFunction;

impl Calculator::Function::Server for PowerFunction {
    fn call(&mut self, mut context : Calculator::Function::CallContext) {
        let (params, results) = context.get();
        let params = params.get_params();
        if params.size() != 2 {
            //"Wrong number of parameters"
            return context.fail();
        };
        results.set_value(params.get(0).powf(params.get(1)));
        context.done();
    }
}

pub fn main() {
    let args = std::os::args();
    if args.len() != 3 {
        println!("usage: {} client HOST:PORT", args[0]);
        return;
    }

    let mut rpc_client = EzRpcClient::new(args[2].as_slice()).unwrap();

    let calculator : Calculator::Client = rpc_client.import_cap("calculator");

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
        request.init().get_expression().set_literal(123.0);

        let eval_promise = request.send();

        let mut read_promise = eval_promise.pipeline.get_value().read_request().send();

        let response = read_promise.wait().unwrap();
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
            request.init().set_op(Calculator::Operator::Add);
            request.send().pipeline.get_func()
        };

        let subtract = {
            let mut request = calculator.get_operator_request();
            request.init().set_op(Calculator::Operator::Subtract);
            request.send().pipeline.get_func()
        };

        let mut request = calculator.evaluate_request();

        let subtract_call = request.init().get_expression().init_call();
        subtract_call.set_function(subtract);
        let subtract_params = subtract_call.init_params(2);
        subtract_params.get(1).set_literal(67.0);

        let add_call = subtract_params.get(0).init_call();
        add_call.set_function(add);
        let add_params = add_call.init_params(2);
        add_params.get(0).set_literal(123.0);
        add_params.get(1).set_literal(45.0);

        let eval_promise = request.send();
        let mut read_promise = eval_promise.pipeline.get_value().read_request().send();
        let response = read_promise.wait().unwrap();
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
            request.init().set_op(Calculator::Operator::Add);
            request.send().pipeline.get_func()
        };

        let multiply = {
            let mut request = calculator.get_operator_request();
            request.init().set_op(Calculator::Operator::Multiply);
            request.send().pipeline.get_func()
        };

        //# Build the request to evaluate 4 * 6
        let mut request = calculator.evaluate_request();

        let multiply_call = request.init().get_expression().init_call();
        multiply_call.set_function(multiply);
        let multiply_params = multiply_call.init_params(2);
        multiply_params.get(0).set_literal(4.0);
        multiply_params.get(1).set_literal(6.0);

        let multiply_result = request.send().pipeline.get_value();

        //# Use the result in two calls that add 3 and 5.

        let mut add3_request = calculator.evaluate_request();
        let add3_call = add3_request.init().get_expression().init_call();
        add3_call.set_function(add.clone());
        let add3_params = add3_call.init_params(2);
        add3_params.get(0).set_previous_result(multiply_result.clone());
        add3_params.get(1).set_literal(3.0);
        let mut add3_promise = add3_request.send().pipeline.get_value().read_request().send();

        let mut add5_request = calculator.evaluate_request();
        let add5_call = add5_request.init().get_expression().init_call();
        add5_call.set_function(add);
        let add5_params = add5_call.init_params(2);
        add5_params.get(0).set_previous_result(multiply_result);
        add5_params.get(1).set_literal(5.0);
        let mut add5_promise = add5_request.send().pipeline.get_value().read_request().send();

        assert!(add3_promise.wait().unwrap().get_value() == 27.0);
        assert!(add5_promise.wait().unwrap().get_value() == 29.0);

        println!("PASS");
    }

    {
        //# Our calculator interface supports defining functions.  Here we use it
        //# to define two functions and then make calls to them as follows:
        //#
        //#   f(x, y) = x * 100 + y
        //#   g(x) = f(x, x + 1) * 2;
        //#   f(12, 34)
        //#   g(21)
        //#
        //# Once again, the whole thing takes only one network round trip.

        println!("Defining functions... ");

        let add = {
            let mut request = calculator.get_operator_request();
            request.init().set_op(Calculator::Operator::Add);
            request.send().pipeline.get_func()
        };

        let multiply = {
            let mut request = calculator.get_operator_request();
            request.init().set_op(Calculator::Operator::Multiply);
            request.send().pipeline.get_func()
        };

        let f = {
            let mut request = calculator.def_function_request();
            let def_function_params = request.init();
            def_function_params.set_param_count(2);
            {
                let add_call = def_function_params.get_body().init_call();
                add_call.set_function(add.clone());
                let add_params = add_call.init_params(2);
                add_params.get(1).set_parameter(1);

                let multiply_call = add_params.get(0).init_call();
                multiply_call.set_function(multiply.clone());
                let multiply_params = multiply_call.init_params(2);
                multiply_params.get(0).set_parameter(0);
                multiply_params.get(1).set_literal(100.0);
            }
            request.send().pipeline.get_func()
        };

        let g = {
            let mut request = calculator.def_function_request();
            let def_function_params = request.init();
            def_function_params.set_param_count(1);
            {
                let multiply_call = def_function_params.get_body().init_call();
                multiply_call.set_function(multiply);
                let multiply_params = multiply_call.init_params(2);
                multiply_params.get(1).set_literal(2.0);

                let f_call = multiply_params.get(0).init_call();
                f_call.set_function(f.clone());
                let f_params = f_call.init_params(2);
                f_params.get(0).set_parameter(0);

                let add_call = f_params.get(1).init_call();
                add_call.set_function(add);
                let add_params = add_call.init_params(2);
                add_params.get(0).set_parameter(0);
                add_params.get(1).set_literal(1.0);
            }
            request.send().pipeline.get_func()
        };

        let mut f_eval_request = calculator.evaluate_request();
        let f_call = f_eval_request.init().init_expression().init_call();
        f_call.set_function(f);
        let f_params = f_call.init_params(2);
        f_params.get(0).set_literal(12.0);
        f_params.get(1).set_literal(34.0);
        let mut f_eval_promise = f_eval_request.send().pipeline.get_value().read_request().send();

        let mut g_eval_request = calculator.evaluate_request();
        let g_call = g_eval_request.init().init_expression().init_call();
        g_call.set_function(g);
        g_call.init_params(1).get(0).set_literal(21.0);
        let mut g_eval_promise = g_eval_request.send().pipeline.get_value().read_request().send();

        assert!(f_eval_promise.wait().unwrap().get_value() == 1234.0);
        assert!(g_eval_promise.wait().unwrap().get_value() == 4244.0);

        println!("PASS")
    }

    {
        //# Make a request that will call back to a function defined locally.
        //#
        //# Specifically, we will compute 2^(4 + 5).  However, exponent is not
        //# defined by the Calculator server.  So, we'll implement the Function
        //# interface locally and pass it to the server for it to use when
        //# evaluating the expression.
        //#
        //# This example requires two network round trips to complete, because the
        //# server calls back to the client once before finishing.  In this
        //# particular case, this could potentially be optimized by using a tail
        //# call on the server side -- see CallContext::tailCall().  However, to
        //# keep the example simpler, we haven't implemented this optimization in
        //# the sample server.

        println!("Using a callback... ");

        let add = {
            let mut request = calculator.get_operator_request();
            request.init().set_op(Calculator::Operator::Add);
            request.send().pipeline.get_func()
        };

        let mut request = calculator.evaluate_request();
        {
            let pow_call = request.init().get_expression().init_call();
            pow_call.set_function(FromServer::new(None::<LocalClient>, box PowerFunction));
            let pow_params = pow_call.init_params(2);
            pow_params.get(0).set_literal(2.0);

            let add_call = pow_params.get(1).init_call();
            add_call.set_function(add);
            let add_params = add_call.init_params(2);
            add_params.get(0).set_literal(4.0);
            add_params.get(1).set_literal(5.0);
        }

        let mut response_promise = request.send().pipeline.get_value().read_request().send();
        let response = response_promise.wait().unwrap();

        assert!(response.get_value() == 512.0);

        println!("PASS");
    }
}
