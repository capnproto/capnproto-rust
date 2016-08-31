// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors
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

use capnp_rpc::{RpcSystem, twoparty, rpc_twoparty_capnp};
use pubsub_capnp::{publisher, subscriber};

use gj::{EventLoop, Promise};

struct SubscriberImpl;

impl subscriber::Server for SubscriberImpl {
    fn push_value(&mut self,
                  params: subscriber::PushValueParams,
                  _results: subscriber::PushValueResults)
        -> Promise<(), ::capnp::Error>
    {
        println!("got: {}", pry!(params.get()).get_value());
        Promise::ok(())
    }
}

pub fn main() {
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {} client HOST:PORT", args[0]);
        return;
    }

    EventLoop::top_level(move |wait_scope| -> Result<(), ::capnp::Error> {
        use std::net::ToSocketAddrs;
        let mut event_port = try!(::gjio::EventPort::new());
        let network = event_port.get_network();
        let addr = try!(args[2].to_socket_addrs()).next().expect("could not parse address");
        let address = network.get_tcp_address(addr);
        let stream = try!(address.connect().wait(wait_scope, &mut event_port));
        let mut rpc_network =
            Box::new(twoparty::VatNetwork::new(stream.clone(), stream,
                                               rpc_twoparty_capnp::Side::Client,
                                               Default::default()));

        let disconnect_promise = rpc_network.on_disconnect();
        let mut rpc_system = RpcSystem::new(rpc_network, None);
        let publisher: publisher::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

        let sub = subscriber::ToClient::new(SubscriberImpl).from_server::<::capnp_rpc::Server>();

        let mut request = publisher.subscribe_request();
        request.get().set_subscriber(sub);

        // Need to make sure not to drop the returned subscription object.
        let _result = request.send().promise.wait(wait_scope, &mut event_port).unwrap();

        disconnect_promise.wait(wait_scope, &mut event_port).unwrap();
        Ok(())

    }).expect("top level error");
}
