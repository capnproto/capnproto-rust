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

#![cfg(test)]

extern crate capnp;
extern crate capnp_rpc;

extern crate futures;
extern crate tokio_core;

extern crate mio_uds;

use capnp::Error;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp, twoparty};

use tokio_core::reactor;
use tokio_core::io::Io;

use std::cell::RefCell;
use std::rc::Rc;

pub mod test_capnp {
  include!(concat!(env!("OUT_DIR"), "/test_capnp.rs"));
}

pub mod impls;
pub mod test_util;

#[test]
fn drop_rpc_system() {
    let core = reactor::Core::new().unwrap();
    let handle = core.handle();

    let (instream, _outstream) = ::mio_uds::UnixStream::pair().unwrap();

    let instream = reactor::PollEvented::new(instream, &handle).unwrap();
    let (reader, writer) = instream.split();

    let network =
        Box::new(twoparty::VatNetwork::new(reader, writer,
                                           rpc_twoparty_capnp::Side::Client,
                                           Default::default()));
    let rpc_system = RpcSystem::new(network, None, handle);
    drop(rpc_system);
//        try!(Promise::<(),Error>::ok(()).wait(wait_scope, &mut event_port));
 //       Ok(())
 //   }).expect("top level error");
}

/*
#[test]
fn drop_import_client_after_disconnect() {
    EventLoop::top_level(|wait_scope| -> Result<(), ::capnp::Error> {
        let mut event_port = try!(gjio::EventPort::new());
        let network = event_port.get_network();
        let (client_stream, server_stream) = try!(network.new_socket_pair());
        let (client_reader, client_writer) = (client_stream.clone(), client_stream);
        let client_network =
            Box::new(twoparty::VatNetwork::new(client_reader, client_writer,
                                               rpc_twoparty_capnp::Side::Client,
                                               Default::default()));
        let mut client_rpc_system = RpcSystem::new(client_network, None);

        let (server_reader, server_writer) = (server_stream.clone(), server_stream);
        let server_network =
            Box::new(twoparty::VatNetwork::new(server_reader, server_writer,
                                               rpc_twoparty_capnp::Side::Server,
                                               Default::default()));

        let bootstrap =
            test_capnp::bootstrap::ToClient::new(impls::Bootstrap).from_server::<::capnp_rpc::Server>();

        let server_rpc_system = RpcSystem::new(server_network, Some(bootstrap.client));

        let client: test_capnp::bootstrap::Client = client_rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
        try!(client.test_interface_request().send().promise.wait(wait_scope, &mut event_port));

        drop(server_rpc_system);

        match client.test_interface_request().send().promise.wait(wait_scope, &mut event_port) {
            Err(ref e) if e.kind == ::capnp::ErrorKind::Disconnected => (),
            _ => panic!("Should have gotten a 'disconnected' error."),
        }

        // At one point, attempting to call again would cause a panic.
        match client.test_interface_request().send().promise.wait(wait_scope, &mut event_port) {
            Err(ref e) if e.kind == ::capnp::ErrorKind::Disconnected => (),
            _ => panic!("Should have gotten a 'disconnected' error."),
        }

        drop(client);
        Ok(())
    }).expect("top level error");
}

fn rpc_top_level<F>(main: F)
    where F: FnOnce(&::gj::WaitScope, ::gjio::EventPort, test_capnp::bootstrap::Client) -> Result<(), Error>,
          F: Send + 'static
{
    EventLoop::top_level(|wait_scope| -> Result<(), Box<::std::error::Error>> {
        let event_port = try!(gjio::EventPort::new());
        let network = event_port.get_network();
        let (join_handle, stream) = try!(network.socket_spawn(|stream, wait_scope, mut event_port| {

            let (reader, writer) = (stream.clone(), stream);
            //let reader = ReadWrapper::new(reader,
            //                             ::std::fs::File::create("/Users/dwrensha/Desktop/client.dat").unwrap());
            //let writer = WriteWrapper::new(writer,
            //                               ::std::fs::File::create("/Users/dwrensha/Desktop/server.dat").unwrap());
            let mut network =
                Box::new(twoparty::VatNetwork::new(reader, writer,
                                                   rpc_twoparty_capnp::Side::Server,
                                                   Default::default()));
            let disconnect_promise = network.on_disconnect();
            let bootstrap =
                test_capnp::bootstrap::ToClient::new(impls::Bootstrap).from_server::<::capnp_rpc::Server>();

            let _rpc_system = RpcSystem::new(network, Some(bootstrap.client));
            try!(disconnect_promise.wait(wait_scope, &mut event_port));
            Ok(())
        }));

        let (reader, writer) = (stream.clone(), stream);

        let network =
            Box::new(twoparty::VatNetwork::new(reader, writer,
                                               rpc_twoparty_capnp::Side::Client,
                                               Default::default()));

        let mut rpc_system = RpcSystem::new(network, None);
        let client: test_capnp::bootstrap::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

        try!(main(wait_scope, event_port, client));
        drop(rpc_system);
        join_handle.join().expect("thread exited unsuccessfully");
        Ok(())
    }).expect("top level error");
}


#[test]
fn do_nothing() {
    rpc_top_level(|_wait_scope, _event_port, _client| {
        Ok(())
    });
}

#[test]
fn basic() {
    rpc_top_level(|wait_scope, mut event_port, client| {
        let response = try!(client.test_interface_request().send().promise.wait(wait_scope, &mut event_port));
        let client = try!(try!(response.get()).get_cap());

        let mut request1 = client.foo_request();
        request1.get().set_i(123);
        request1.get().set_j(true);
        let promise1 = request1.send();

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

        let mut request2 = client.baz_request();
        ::test_util::init_test_message(try!(request2.get().get_s()));
        let promise2 = request2.send();

        let response1 = try!(promise1.promise.wait(wait_scope, &mut event_port));
        if try!(try!(response1.get()).get_x()) != "foo" {
            return Err(Error::failed("expected X to equal 'foo'".to_string()));
        }
        try!(promise2.promise.wait(wait_scope, &mut event_port));
        try!(promise3.wait(wait_scope, &mut event_port));
        Ok(())
    });
}

#[test]
fn pipelining() {
    rpc_top_level(|wait_scope, mut event_port, client| {
        let response = try!(client.test_pipeline_request().send().promise.wait(wait_scope, &mut event_port));
        let client = try!(try!(response.get()).get_cap());

        let mut request = client.get_cap_request();
        request.get().set_n(234);
        let server = impls::TestInterface::new();
        let chained_call_count = server.get_call_count();
        request.get().set_in_cap(
            ::test_capnp::test_interface::ToClient::new(server).from_server::<::capnp_rpc::Server>());

        let promise = request.send();

        let mut pipeline_request = promise.pipeline.get_out_box().get_cap().foo_request();
        pipeline_request.get().set_i(321);
        let pipeline_promise = pipeline_request.send();

        let pipeline_request2 = {
            let extends_client =
                ::test_capnp::test_extends::Client { client: promise.pipeline.get_out_box().get_cap().client };
            extends_client.grault_request()
        };
        let pipeline_promise2 = pipeline_request2.send();

        drop(promise); // Just to be annoying, drop the original promise.

        if chained_call_count.get() != 0 {
            return Err(Error::failed("expected chained_call_count to equal 0".to_string()));
        }

        let response = try!(pipeline_promise.promise.wait(wait_scope, &mut event_port));
        if try!(try!(response.get()).get_x()) != "bar" {
            return Err(Error::failed("expected x to equal 'bar'".to_string()));
        }

        let response2 = try!(pipeline_promise2.promise.wait(wait_scope, &mut event_port));
        ::test_util::CheckTestMessage::check_test_message(try!(response2.get()));
        assert_eq!(chained_call_count.get(), 1);
        Ok(())
    });
}

#[test]
fn pipelining_return_null() {
    rpc_top_level(|wait_scope, mut event_port, client| {
        let response = try!(client.test_pipeline_request().send().promise.wait(wait_scope, &mut event_port));
        let client = try!(try!(response.get()).get_cap());

        let request = client.get_null_cap_request();
        let cap = request.send().pipeline.get_cap();
        match cap.foo_request().send().promise.wait(wait_scope, &mut event_port) {
            Err(ref e) => {
                if e.description.contains("Message contains null capability pointer") {
                    Ok(())
                } else {
                    Err(Error::failed(format!("Should have gotten null capability error. Instead got {:?}", e)))
                }
            }
            Ok(_) => {
                Err(Error::failed(format!("Should have gotten null capability error.")))
            }
        }
    });
}

#[test]
fn null_capability() {
    let mut message = ::capnp::message::Builder::new_default();
    let root: ::test_capnp::test_all_types::Builder = message.get_root().unwrap();

    // In capnproto-c++, this would return a BrokenCap. Here, it returns a decode error.
    // Would it be worthwhile to try to match the C++ behavior here? We would need something
    // like the BrokenCapFactory singleton.
    assert!(root.get_interface_field().is_err());
}

#[test]
fn release_simple() {
    rpc_top_level(|wait_scope, mut event_port, client| {
        let response = try!(client.test_more_stuff_request().send().promise.wait(wait_scope, &mut event_port));
        let client = try!(try!(response.get()).get_cap());

        let handle1 = client.get_handle_request().send().promise;
        let ::capnp::capability::RemotePromise {promise, pipeline} = client.get_handle_request().send();
        let handle2 = try!(try!(try!(promise.wait(wait_scope, &mut event_port)).get()).get_handle());

        let get_count_response = try!(client.get_handle_count_request().send().promise.wait(wait_scope, &mut event_port));
        if try!(get_count_response.get()).get_count() != 2 {
            return Err(Error::failed("expected handle count to equal 2".to_string()))
        }

        drop(handle1);

        let get_count_response = try!(client.get_handle_count_request().send().promise.wait(wait_scope, &mut event_port));
        if try!(get_count_response.get()).get_count() != 1 {
            return Err(Error::failed("expected handle count to equal 1".to_string()))
        }

        drop(handle2);

        let get_count_response = try!(client.get_handle_count_request().send().promise.wait(wait_scope, &mut event_port));
        if try!(get_count_response.get()).get_count() != 1 {
            return Err(Error::failed("expected handle count to equal 1".to_string()))
        }

        drop(pipeline);

        let get_count_response = try!(client.get_handle_count_request().send().promise.wait(wait_scope, &mut event_port));
        if try!(get_count_response.get()).get_count() != 0 {
            return Err(Error::failed("expected handle count to equal 0".to_string()))
        }

        Ok(())
    });
}

#[test]
fn release_on_cancel() {
    rpc_top_level(|wait_scope, mut event_port, client| {
        let response = try!(client.test_more_stuff_request().send().promise.wait(wait_scope, &mut event_port));
        let client = try!(try!(response.get()).get_cap());

        let promise = client.get_handle_request().send();

        // If the server receives cancellation too early, it won't even return a capability in the
        // results, it will just return "canceled". We want to emulate the case where the return message
        // and the cancel (finish) message cross paths.

        let _ = Promise::<(), ::std::io::Error>::ok(()).map(|()| Ok(())).wait(wait_scope, &mut event_port);
        let _ = Promise::<(), ::std::io::Error>::ok(()).map(|()| Ok(())).wait(wait_scope, &mut event_port);
        drop(promise);

        for _ in 0..16 {
            let _ = Promise::<(), ::std::io::Error>::ok(()).map(|()| Ok(())).wait(wait_scope, &mut event_port);
        }

        let get_count_response = try!(client.get_handle_count_request().send().promise.wait(wait_scope, &mut event_port));
        let handle_count = try!(get_count_response.get()).get_count();
        if handle_count != 0 {
            return Err(Error::failed(format!("handle count: expected 0, but got {}", handle_count)))
        }

        Ok(())
    });
}

#[test]
fn promise_resolve() {
    rpc_top_level(|wait_scope, mut event_port, client| {
        let response = try!(client.test_more_stuff_request().send().promise.wait(wait_scope, &mut event_port));
        let client = try!(try!(response.get()).get_cap());

        let mut request = client.call_foo_request();
        let mut request2 = client.call_foo_when_resolved_request();

        let (paf_promise, paf_fulfiller) =
            Promise::<::test_capnp::test_interface::Client, Error>::and_fulfiller();

        {
            let mut fork = paf_promise.fork();
            let cap1 = ::capnp_rpc::new_promise_client(fork.add_branch().map(|c| Ok(c.client)));
            let cap2 = ::capnp_rpc::new_promise_client(fork.add_branch().map(|c| Ok(c.client)));
            request.get().set_cap(cap1);
            request2.get().set_cap(cap2);
        }

        let promise = request.send().promise;
        let promise2 = request2.send().promise;

        // Make sure getCap() has been called on the server side by sending another call and waiting
        // for it.
        let client2 = ::test_capnp::test_call_order::Client { client: client.clone().client };
        let _response = try!(client2.get_call_sequence_request().send().promise.wait(wait_scope, &mut event_port));

        let server = impls::TestInterface::new();
        paf_fulfiller.fulfill(
            ::test_capnp::test_interface::ToClient::new(server).from_server::<::capnp_rpc::Server>());

        let response = try!(promise.wait(wait_scope, &mut event_port));
        if try!(try!(response.get()).get_s()) != "bar" {
            return Err(Error::failed("expected s to equal 'bar'".to_string()));
        }
        let response = try!(promise2.wait(wait_scope, &mut event_port));
        if try!(try!(response.get()).get_s()) != "bar" {
            return Err(Error::failed("expected s to equal 'bar'".to_string()));
        }
        Ok(())
    });
}

#[test]
fn retain_and_release() {
    use std::cell::Cell;
    use std::rc::Rc;

    rpc_top_level(|wait_scope, mut event_port, client| {
        let (promise, fulfiller) = Promise::<(), Error>::and_fulfiller();
        let destroyed = Rc::new(Cell::new(false));

        let destroyed1 = destroyed.clone();
        let destruction_promise = promise.map(move |()| {
            destroyed1.set(true);
            Ok(())
        }).eagerly_evaluate();

        {
            let response = try!(client.test_more_stuff_request().send().promise.wait(wait_scope, &mut event_port));
            let client = try!(try!(response.get()).get_cap());

            {
                let mut request = client.hold_request();
                request.get().set_cap(
                    ::test_capnp::test_interface::ToClient::new(impls::TestCapDestructor::new(fulfiller))
                        .from_server::<::capnp_rpc::Server>());
                try!(request.send().promise.wait(wait_scope, &mut event_port));
            }

            // Do some other call to add a round trip.
            // ugh, we need upcasting.
            let client1 = ::test_capnp::test_call_order::Client { client: client.clone().client };
            let response = try!(client1.get_call_sequence_request().send().promise.wait(wait_scope, &mut event_port));
            if try!(response.get()).get_n() != 1 {
                return Err(Error::failed("N should equal 1".to_string()))
            }

            if destroyed.get() {
                return Err(Error::failed("shouldn't be destroyed yet".to_string()))
            }

            // We can ask it to call the held capability.
            let response = try!(client.call_held_request().send().promise.wait(wait_scope, &mut event_port));
            if try!(try!(response.get()).get_s()) != "bar" {
                return Err(Error::failed("S should equal 'bar'".to_string()))
            }

            {
                // we can get the cap back from it.
                let response = try!(client.get_held_request().send().promise.wait(wait_scope, &mut event_port));
                let cap_copy = try!(try!(response.get()).get_cap());

                // And call it, without any network communications.
                // (TODO: verify that no network communications happen here)
                {
                    let mut request = cap_copy.foo_request();
                    request.get().set_i(123);
                    request.get().set_j(true);
                    let response = try!(request.send().promise.wait(wait_scope, &mut event_port));
                    if try!(try!(response.get()).get_x()) != "foo" {
                        return Err(Error::failed("X should equal 'foo'.".to_string()));
                    }
                }

                {
                    // We can send another copy of the same cap to another method, and it works.
                    let mut request = client.call_foo_request();
                    request.get().set_cap(cap_copy);
                    let response = try!(request.send().promise.wait(wait_scope, &mut event_port));
                    if try!(try!(response.get()).get_s()) != "bar" {
                        return Err(Error::failed("S should equal 'bar'.".to_string()));
                    }
                }
            }

            // Give some time to settle.
            let response = try!(client1.get_call_sequence_request().send().promise.wait(wait_scope, &mut event_port));
            if try!(response.get()).get_n() != 5 {
                return Err(Error::failed("N should equal 5.".to_string()));
            }
            let response = try!(client1.get_call_sequence_request().send().promise.wait(wait_scope, &mut event_port));
            if try!(response.get()).get_n() != 6 {
                return Err(Error::failed("N should equal 6.".to_string()));
            }
            let response = try!(client1.get_call_sequence_request().send().promise.wait(wait_scope, &mut event_port));
            if try!(response.get()).get_n() != 7 {
                return Err(Error::failed("N should equal 7.".to_string()));
            }

            if destroyed.get() {
                return Err(Error::failed("haven't released it yet".to_string()))
            }
        }

        try!(destruction_promise.wait(wait_scope, &mut event_port));
        if !destroyed.get() {
            return Err(Error::failed("should be destroyed now".to_string()));
        }
        Ok(())
    });
}


#[test]
fn cancel() {
    use std::rc::Rc;
    use std::cell::Cell;

    rpc_top_level(|wait_scope, mut event_port, client| {
        let response = try!(client.test_more_stuff_request().send().promise.wait(wait_scope, &mut event_port));
        let client = try!(try!(response.get()).get_cap());

        let (promise, fulfiller) = Promise::and_fulfiller();
        let destroyed = Rc::new(Cell::new(false));
        let destroyed1 = destroyed.clone();
        let destruction_promise = promise.map(move |()| {
            destroyed1.set(true);
            Ok(())
        }).eagerly_evaluate();

        {
            let mut request = client.never_return_request();
            request.get().set_cap(
                ::test_capnp::test_interface::ToClient::new(impls::TestCapDestructor::new(fulfiller))
                    .from_server::<::capnp_rpc::Server>());

            {
                let _response_promise = request.send();

                // Allow some time to settle.

                // ugh, we need upcasting.
                let client = ::test_capnp::test_call_order::Client { client: client.client };
                let response = try!(client.get_call_sequence_request().send().promise.wait(wait_scope, &mut event_port));
                if try!(response.get()).get_n() != 1 {
                    return Err(Error::failed("N should equal 1.".to_string()));
                }
                let response = try!(client.get_call_sequence_request().send().promise.wait(wait_scope, &mut event_port));
                if try!(response.get()).get_n() != 2 {
                    return Err(Error::failed("N should equal 2.".to_string()));
                }
                if destroyed.get() {
                        return Err(Error::failed("Shouldn't be destroyed yet.".to_string()));
                }
            }
        }
        try!(destruction_promise.wait(wait_scope, &mut event_port));
        if !destroyed.get() {
            return Err(Error::failed("The cap should be released now.".to_string()));
        }
        Ok(())
    });
}


#[test]
fn dont_hold() {
    rpc_top_level(|wait_scope, mut event_port, client| {
        let response = try!(client.test_more_stuff_request().send().promise.wait(wait_scope, &mut event_port));
        let client = try!(try!(response.get()).get_cap());

        let (promise, fulfiller) =
            Promise::<::test_capnp::test_interface::Client, Error>::and_fulfiller();

        let cap: ::test_capnp::test_interface::Client =
            ::capnp_rpc::new_promise_client(promise.map(|c| Ok(c.client)));

        let mut request = client.dont_hold_request();
        request.get().set_cap(cap.clone());
        request.send().promise.then(move |_response| {
            let mut request = client.dont_hold_request();
            request.get().set_cap(cap.clone());
            request.send().promise.then(move |_| {
                drop(fulfiller);
                Promise::ok(())
            })
        }).wait(wait_scope, &mut event_port)
    });
}

fn get_call_sequence(client: &::test_capnp::test_call_order::Client, expected: u32)
                     -> ::capnp::capability::RemotePromise<::test_capnp::test_call_order::get_call_sequence_results::Owned>
{
    let mut req = client.get_call_sequence_request();
    req.get().set_expected(expected);
    req.send()
}

#[test]
fn embargo_success() {
    rpc_top_level(|wait_scope, mut event_port, client| {
        let response = try!(client.test_more_stuff_request().send().promise.wait(wait_scope, &mut event_port));
        let client = try!(try!(response.get()).get_cap());

        let server = ::impls::TestCallOrder::new();

        // ugh, we need upcasting.
        let client2 = ::test_capnp::test_call_order::Client { client: client.clone().client };
        let early_call = client2.get_call_sequence_request().send();
        drop(client2);

        let mut echo_request = client.echo_request();
        echo_request.get().set_cap(
            ::test_capnp::test_call_order::ToClient::new(server).from_server::<::capnp_rpc::Server>());
        let echo = echo_request.send();

        let pipeline = echo.pipeline.get_cap();

        let call0 = get_call_sequence(&pipeline, 0);
        let call1 = get_call_sequence(&pipeline, 1);

        try!(early_call.promise.wait(wait_scope, &mut event_port));

        let call2 = get_call_sequence(&pipeline, 2);

        let _resolved = try!(echo.promise.wait(wait_scope, &mut event_port));

        let call3 = get_call_sequence(&pipeline, 3);
        let call4 = get_call_sequence(&pipeline, 4);
        let call5 = get_call_sequence(&pipeline, 5);

        Promise::all(vec![call0.promise,
                          call1.promise,
                          call2.promise,
                          call3.promise,
                          call4.promise,
                          call5.promise].into_iter()).map(|responses| {
            let mut counter = 0;
            for r in responses.into_iter() {
                if counter != try!(r.get()).get_n() {
                    return Err(Error::failed(
                        "calls arrived out of order".to_string()))
                }
                counter += 1;
            }
            Ok(())
        }).wait(wait_scope, &mut event_port)
    });
}

fn expect_promise_throws<T>(promise: Promise<T, Error>, wait_scope: &::gj::WaitScope,
                            event_port: &mut ::gjio::EventPort)
                            -> Result<(), Error> {
    promise.map_else(|r| match r {
        Ok(_) => Err(Error::failed("expected promise to fail".to_string())),
        Err(_) => Ok(()),
    }).wait(wait_scope, event_port)
}

#[test]
fn embargo_error() {
    rpc_top_level(|wait_scope, mut event_port, client| {
        let response = try!(client.test_more_stuff_request().send().promise.wait(wait_scope, &mut event_port));
        let client = try!(try!(response.get()).get_cap());

        let (promise, fulfiller) =
            Promise::<::test_capnp::test_call_order::Client, Error>::and_fulfiller();

        let cap = ::capnp_rpc::new_promise_client(promise.map(|c| Ok(c.client)));

        // ugh, we need upcasting.
        let client2 = ::test_capnp::test_call_order::Client { client: client.clone().client };
        let early_call = client2.get_call_sequence_request().send();
        drop(client2);

        let mut echo_request = client.echo_request();
        echo_request.get().set_cap(cap);
        let echo = echo_request.send();

        let pipeline = echo.pipeline.get_cap();

        let call0 = get_call_sequence(&pipeline, 0);
        let call1 = get_call_sequence(&pipeline, 1);

        try!(early_call.promise.wait(wait_scope, &mut event_port));

        let call2 = get_call_sequence(&pipeline, 2);

        let _resolved = echo.promise.wait(wait_scope, &mut event_port);

        let call3 = get_call_sequence(&pipeline, 3);
        let call4 = get_call_sequence(&pipeline, 4);
        let call5 = get_call_sequence(&pipeline, 5);

        fulfiller.reject(Error::failed("foo".to_string()));

        try!(expect_promise_throws(call0.promise, wait_scope, &mut event_port));
        try!(expect_promise_throws(call1.promise, wait_scope, &mut event_port));
        try!(expect_promise_throws(call2.promise, wait_scope, &mut event_port));
        try!(expect_promise_throws(call3.promise, wait_scope, &mut event_port));
        try!(expect_promise_throws(call4.promise, wait_scope, &mut event_port));
        try!(expect_promise_throws(call5.promise, wait_scope, &mut event_port));
        Ok(())
    });
}

#[test]
fn echo_destruction() {
    rpc_top_level(|wait_scope, mut event_port, client| {
        let response = try!(client.test_more_stuff_request().send().promise.wait(wait_scope, &mut event_port));
        let client = try!(try!(response.get()).get_cap());

        let (promise, fulfiller) =
            Promise::<::test_capnp::test_call_order::Client, Error>::and_fulfiller();

        let cap = ::capnp_rpc::new_promise_client(promise.map(|c| Ok(c.client)));

        // ugh, we need upcasting.
        let client2 = ::test_capnp::test_call_order::Client { client: client.clone().client };
        let early_call = client2.get_call_sequence_request().send();
        drop(client2);

        let mut echo_request = client.echo_request();
        echo_request.get().set_cap(cap);
        let echo = echo_request.send();

        let pipeline = echo.pipeline.get_cap();

        early_call.promise.then(move |_early_call_response| {
            let _ = get_call_sequence(&pipeline, 2);
            echo.promise.then(move |_echo_response| {
                fulfiller.reject(Error::failed("foo".to_string()));
                Promise::ok(())
            })
        }).wait(wait_scope, &mut event_port)
    })
}
*/
