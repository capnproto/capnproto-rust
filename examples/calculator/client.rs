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

use capnp_rpc::{rpc, twoparty, rpc_twoparty_capnp};
use calculator_capnp::calculator;
use gj::Promise;

pub fn main() {
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {} client HOST:PORT", args[0]);
        return;
    }

    ::gj::EventLoop::top_level(move |wait_scope| {
        use std::net::ToSocketAddrs;
        let addr = try!(args[2].to_socket_addrs()).next().expect("could not parse address");
        let stream = try!(::gj::io::tcp::Stream::connect(addr).wait(wait_scope));
        let stream2 = try!(stream.try_clone());
        let connection: Box<::capnp_rpc::VatNetwork<twoparty::VatId>> =
            Box::new(twoparty::VatNetwork::new(stream, stream2, Default::default()));
        let mut rpc_system = rpc::System::new(connection, None);
        let calculator = calculator::Client {
            client: rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server)
        };

        {
            println!("Evaluating a literal...");
            let mut request = calculator.evaluate_request();
            request.init().init_expression().set_literal(11.0);
            request.send().promise.then(|response| {
                let value = pry!(pry!(response.get()).get_value());
                let request = value.read_request();
                request.send().promise.then(|response|{
                    assert_eq!(pry!(response.get()).get_value(), 11.0);
                    Promise::ok(())
                })
            }).wait(wait_scope).unwrap();
            println!("PASS");
        }

        {
            println!("Evaluating a literal using pipelining...");
            let mut request = calculator.evaluate_request();
            request.init().init_expression().set_literal(23.0);
            let value = request.send().pipeline.get_value();
            let request = value.read_request();
            request.send().promise.then(|response|{
                assert_eq!(pry!(response.get()).get_value(), 23.0);
                Promise::ok(())
            }).wait(wait_scope).unwrap();
            println!("PASS");
        }

        {
            // Make a request to evaluate 123 + 45 - 67.
            //
            // The Calculator interface requires that we first call getOperator() to
            // get the addition and subtraction functions, then call evaluate() to use
            // them.  But, once again, we can get both functions, call evaluate(), and
            // then read() the result -- four RPCs -- in the time of *one* network
            // round trip, because of promise pipelining.

            println!("Using add and subtract... ");

            let add = {
                // Get the "add" function from the server.
                let mut request = calculator.get_operator_request();
                request.init().set_op(calculator::Operator::Add);
                request.send().pipeline.get_func()
            };

            let subtract = {
                // Get the "subtract" function from the server.
                let mut request = calculator.get_operator_request();
                request.init().set_op(calculator::Operator::Subtract);
                request.send().pipeline.get_func()
            };

            // Build the request to evaluate 123 + 45 - 67.
            let mut request = calculator.evaluate_request();

            {
                let mut subtract_call = request.init().init_expression().init_call();
                subtract_call.set_function(subtract);
                let mut subtract_params = subtract_call.init_params(2);
                subtract_params.borrow().get(1).set_literal(67.0);

                let mut add_call = subtract_params.get(0).init_call();
                add_call.set_function(add);
                let mut add_params = add_call.init_params(2);
                add_params.borrow().get(0).set_literal(123.0);
                add_params.get(1).set_literal(45.0);
            }

            // Send the evaluate() request, read() the result, and wait for read() to
            // finish.
            let eval_promise = request.send();
            let read_promise = eval_promise.pipeline.get_value().read_request().send();

            let response = try!(read_promise.promise.wait(wait_scope));
            assert_eq!(try!(response.get()).get_value(), 101.0);

            println!("PASS");
        }

        Ok(())
    }).expect("top level error");
}
