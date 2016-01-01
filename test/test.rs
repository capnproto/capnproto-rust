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

pub mod test_capnp {
  include!(concat!(env!("OUT_DIR"), "/test_capnp.rs"));
}

#[test]
fn drop_rpc_system() {
    EventLoop::top_level(|wait_scope| {
        let (instream, outstream) = try!(unix::Stream::new_pair());
        let network =
            Box::new(twoparty::VatNetwork::new(instream, outstream,
                                               rpc_twoparty_capnp::Side::Client,
                                               Default::default()));
        let rpc_system = rpc::System::new(network, None);
        drop(rpc_system);
        try!(Promise::<(),Error>::ok(()).wait(wait_scope));
        Ok(())
    }).expect("top level error");
}

fn set_up_rpc<F>(main: F)
    where F: FnOnce(rpc::System<twoparty::VatId>) -> Promise<(), Error>, F: Send + 'static
{
    EventLoop::top_level(|wait_scope| {
        let (join_handle, stream) = try!(unix::spawn(|stream, wait_scope| {
            let stream2 = try!(stream.try_clone());
            let network =
                Box::new(twoparty::VatNetwork::new(stream, stream2,
                                                   rpc_twoparty_capnp::Side::Client,
                                                   Default::default()));

            let rpc_system = rpc::System::new(network, None);
            try!(main(rpc_system).wait(wait_scope));
            Ok(())
        }));


        let stream2 = try!(stream.try_clone());
        let mut network =
            Box::new(twoparty::VatNetwork::new(stream, stream2,
                                               rpc_twoparty_capnp::Side::Server,
                                               Default::default()));
        let disconnect_promise = network.on_disconnect();
        let _rpc_system = rpc::System::new(network, None);

        try!(disconnect_promise.wait(wait_scope));
        join_handle.join().expect("thread exited unsuccessfully");
        Ok(())
    }).expect("top level error");
}

/*
#[test]
fn do_nothing() {
    set_up_rpc(|_rpc_system| {
        Promise::ok(())
    });
}*/


