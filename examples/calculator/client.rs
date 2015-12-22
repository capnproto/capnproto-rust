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
                let mut request = value.read_request();
                request.send().promise.then(|response|{
                    assert_eq!(pry!(response.get()).get_value(), 11.0);
                    Promise::ok(())
                })
            }).wait(wait_scope).unwrap();
            println!("PASS");
        }

/*        {
            println!("Evaluating a literal using pipelining...");
            let mut request = calculator.evaluate_request();
            request.init().init_expression().set_literal(23.0);
            let value = request.send().pipeline.get_value();
            let mut request = value.read_request();
            request.send().promise.then(|response|{
                assert_eq!(pry!(response.get()).get_value(), 23.0);
                Promise::ok(())
            }).wait(wait_scope).unwrap();
            println!("PASS");
        }*/

        Ok(())
    }).expect("top level error");
}
