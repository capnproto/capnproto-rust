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

extern crate capnp;
extern crate capnp_rpc;

#[macro_use]
extern crate gj;

use gj::{EventLoop, Promise};
use gj::io::unix;
use capnp::Error;
use capnp_rpc::{rpc, rpc_twoparty_capnp, twoparty};
use capnp_rpc::rpc::LocalClient;

pub mod test_capnp {
  include!(concat!(env!("OUT_DIR"), "/test_capnp.rs"));
}

pub mod impls;

#[test]
fn drop_rpc_system() {
    EventLoop::top_level(|wait_scope| {
        let (instream, _outstream) = try!(unix::Stream::new_pair());
        let (reader, writer) = instream.split();
        let network =
            Box::new(twoparty::VatNetwork::new(reader, writer,
                                               rpc_twoparty_capnp::Side::Client,
                                               Default::default()));
        let rpc_system = rpc::System::new(network, None);
        drop(rpc_system);
        try!(Promise::<(),Error>::ok(()).wait(wait_scope));
        Ok(())
    }).expect("top level error");
}

fn set_up_rpc<F>(main: F)
    where F: FnOnce(test_capnp::bootstrap::Client) -> Promise<(), Error>,
          F: Send + 'static
{
    EventLoop::top_level(|wait_scope| {
        let (join_handle, stream) = try!(unix::spawn(|stream, wait_scope| {
            let (reader, writer) = stream.split();
            let network =
                Box::new(twoparty::VatNetwork::new(reader, writer,
                                                   rpc_twoparty_capnp::Side::Client,
                                                   Default::default()));

            let mut rpc_system = rpc::System::new(network, None);
            let client = test_capnp::bootstrap::Client {
                client: rpc_system.bootstrap(rpc_twoparty_capnp::Side::Client)
            };

            try!(main(client).wait(wait_scope));
            Ok(())
        }));

        let (reader, writer) = stream.split();
        let mut network =
            Box::new(twoparty::VatNetwork::new(reader, writer,
                                               rpc_twoparty_capnp::Side::Server,
                                               Default::default()));
        let disconnect_promise = network.on_disconnect();

        let bootstrap =
            test_capnp::bootstrap::ToClient::new(impls::Bootstrap).from_server::<LocalClient>();

        let _rpc_system = rpc::System::new(network, Some(bootstrap.client));

        try!(disconnect_promise.wait(wait_scope));
        join_handle.join().expect("thread exited unsuccessfully");
        Ok(())
    }).expect("top level error");
}


#[test]
fn do_nothing() {
    set_up_rpc(|_client| {
        Promise::ok(())
    });
}

#[test]
fn basic() {
    set_up_rpc(|client| {
        client.test_interface_request().send().promise.then(|response| {
            let client = pry!(pry!(response.get()).get_cap());
            let mut request1 = client.foo_request();
            request1.get().set_i(123);
            request1.get().set_j(true);
            let promise1 = request1.send().promise.then(|response| {
                if "foo" == pry!(pry!(response.get()).get_x()) {
                    Promise::ok(())
                } else {
                    Promise::err(Error::failed("expected X to equal 'foo'".to_string()))
                }
            });

            let request3 = client.bar_request();
            let promise3 = request3.send().promise.then_else(|result| {
                // We expect this call to fail.
                match result {
                    Ok(_) => {
                        Promise::err(Error::failed("expected bar() to fail".to_string()))
                    }
                    Err(_) => {
                        Promise::ok(())
                    }
                }
            });

            let request2 = client.baz_request();
            // TODO fill in some values and check that they are faithfully sent.
            let promise2 = request2.send().promise.map(|_| Ok(()));

            Promise::all(vec![promise1, promise2, promise3].into_iter()).map(|_| Ok(()))
        })
    });
}


