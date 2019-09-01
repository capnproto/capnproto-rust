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

#[macro_use]
extern crate capnp_rpc;

extern crate futures;
extern crate romio;

use capnp::Error;
use capnp::capability::Promise;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp, twoparty};

use futures::{Future, FutureExt, TryFutureExt};
use futures::task::LocalSpawn;
use futures::channel::oneshot;

use futures::{AsyncReadExt};

pub mod test_capnp {
  include!(concat!(env!("OUT_DIR"), "/test_capnp.rs"));
}


pub mod impls;
pub mod test_util;

fn canceled_to_error(_e: futures::channel::oneshot::Canceled) -> Error {
        Error::failed(format!("oneshot was canceled"))
}

#[test]
fn drop_rpc_system() {
    let (instream, _outstream) = romio::uds::UnixStream::pair().expect("socket pair");
    let (reader, writer) = instream.split();

    let network =
        Box::new(twoparty::VatNetwork::new(reader, writer,
                                           rpc_twoparty_capnp::Side::Client,
                                           Default::default()));
    let rpc_system = RpcSystem::new(network, None);
    drop(rpc_system);
    let mut exec = futures::executor::LocalPool::new();
    exec.run_until_stalled();
}

fn disconnector_setup() -> ( RpcSystem<capnp_rpc::rpc_twoparty_capnp::Side>, RpcSystem<capnp_rpc::rpc_twoparty_capnp::Side> ) {
    let (client_stream, server_stream) = romio::uds::UnixStream::pair().expect("socket pair");
    let (client_reader, client_writer) = client_stream.split();

    let client_network =
        Box::new(twoparty::VatNetwork::new(client_reader, client_writer,
                                           rpc_twoparty_capnp::Side::Client,
                                           Default::default()));

    let client_rpc_system = RpcSystem::new(client_network, None);

    let (server_reader, server_writer) = server_stream.split();

    let server_network =
        Box::new(twoparty::VatNetwork::new(server_reader, server_writer,
                                           rpc_twoparty_capnp::Side::Server,
                                           Default::default()));

    let bootstrap =
        test_capnp::bootstrap::ToClient::new(impls::Bootstrap).into_client::<::capnp_rpc::Server>();

    let server_rpc_system = RpcSystem::new(server_network, Some(bootstrap.client));

    ( client_rpc_system, server_rpc_system )
}

fn spawn<F>(spawner: &mut futures::executor::LocalSpawner, task: F)
    where F: Future<Output = Result<(), Error>> + 'static,
{
    spawner.spawn_local_obj(
        Box::pin(task.map(|r| {
            if let Err(e) = r {
                panic!("Error on spawned task: {:?}", e);
            }
        })).into()).expect("spawn")
}

#[test]
fn drop_import_client_after_disconnect() {
    let (mut client_rpc_system, server_rpc_system) = disconnector_setup();

    let client: test_capnp::bootstrap::Client = client_rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

    let mut exec = futures::executor::LocalPool::new();
    let mut spawner = exec.spawner();
    spawn(&mut spawner, client_rpc_system);

    let (tx, rx) = oneshot::channel::<()>();
    let rx = rx.map_err(crate::canceled_to_error);
    spawn(&mut spawner, futures::future::try_join(rx, server_rpc_system).map(|_|Ok(())));

    exec.run_until(client.test_interface_request().send().promise).unwrap();

    drop(tx);

    match exec.run_until(client.test_interface_request().send().promise) {
        Err(ref e) if e.kind == ::capnp::ErrorKind::Disconnected => (),
        Err(e) => panic!("wrong kind of error: {:?}", e),
        _ => panic!("Should have gotten a 'disconnected' error."),
    }

    // At one point, attempting to call again would cause a panic.
    match exec.run_until(client.test_interface_request().send().promise) {
        Err(ref e) if e.kind == ::capnp::ErrorKind::Disconnected => (),
        _ => panic!("Should have gotten a 'disconnected' error."),
    }

    drop(client);
}

#[test]
fn disconnector_disconnects() {
    let (mut client_rpc_system, server_rpc_system) = disconnector_setup();

    let client: test_capnp::bootstrap::Client = client_rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
    let disconnector: capnp_rpc::Disconnector<capnp_rpc::rpc_twoparty_capnp::Side> = client_rpc_system.get_disconnector();

    let mut exec = futures::executor::LocalPool::new();
    let mut spawner = exec.spawner();
    spawn(&mut spawner, client_rpc_system);

    let (tx, rx) = oneshot::channel::<()>();

    //send on tx when server_rpc_system exits
    spawn(&mut spawner, server_rpc_system.map(|x| {let _ = tx.send(()).expect("sending on tx"); x}));

    //make sure we can make an RPC system call
    exec.run_until(client.test_interface_request().send().promise).unwrap();

    //disconnect from the server; comment this next line out to see the test fail
    exec.run_until(disconnector).unwrap();

    exec.run_until(rx).expect("rpc system should exit");

    //make sure we can't use client any more (because the server is disconnected)
    match exec.run_until(client.test_interface_request().send().promise) {
        Err(ref e) if e.kind == ::capnp::ErrorKind::Disconnected => (),
        _ => panic!("Should have gotten a 'disconnected' error."),
    }
}

fn rpc_top_level<F>(main: F)
    where F: FnOnce(futures::executor::LocalPool, test_capnp::bootstrap::Client) -> Result<(), Error>,
          F: Send + 'static
{
    let (client_stream, server_stream) = romio::uds::UnixStream::pair().expect("socket pair");

    let join_handle = ::std::thread::spawn(move || {
        let (server_reader, server_writer) = server_stream.split();

        let network =
            Box::new(twoparty::VatNetwork::new(server_reader, server_writer,
                                               rpc_twoparty_capnp::Side::Server,
                                               Default::default()));

        let bootstrap =
            test_capnp::bootstrap::ToClient::new(impls::Bootstrap).into_client::<::capnp_rpc::Server>();

        let rpc_system = RpcSystem::new(network, Some(bootstrap.client));
        let mut exec = futures::executor::LocalPool::new();
        exec.run_until(rpc_system).unwrap();
    });

    let (client_reader, client_writer) = client_stream.split();

    let network =
        Box::new(twoparty::VatNetwork::new(client_reader, client_writer,
                                           rpc_twoparty_capnp::Side::Client,
                                           Default::default()));

    let mut rpc_system = RpcSystem::new(network, None);
    let client: test_capnp::bootstrap::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

    let exec = futures::executor::LocalPool::new();
    let mut spawner = exec.spawner();
    spawn(&mut spawner, rpc_system);

    main(exec, client).unwrap();
    join_handle.join().expect("thread exited unsuccessfully");
}

#[test]
fn do_nothing() {
    rpc_top_level(|_core, _client| {
        Ok(())
    });
}

#[test]
fn basic_rpc_calls() {
    rpc_top_level(|mut exec, client| {
        let response = exec.run_until(client.test_interface_request().send().promise)?;
        let client = response.get()?.get_cap()?;

        let mut request1 = client.foo_request();
        request1.get().set_i(123);
        request1.get().set_j(true);
        let promise1 = request1.send();

        let request3 = client.bar_request();
        let promise3 = request3.send().promise.then(|result| {
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

        crate::test_util::init_test_message(request2.get().get_s()?);
        let promise2 = request2.send();

        let response1 = exec.run_until(promise1.promise)?;

        if response1.get()?.get_x()? != "foo" {
            return Err(Error::failed("expected X to equal 'foo'".to_string()));
        }

        exec.run_until(promise2.promise)?;
        exec.run_until(promise3)?;
        Ok(())
    });
}

#[test]
fn basic_pipelining() {
    rpc_top_level(|mut exec, client| {
        let response = exec.run_until(client.test_pipeline_request().send().promise)?;
        let client = response.get()?.get_cap()?;

        let mut request = client.get_cap_request();
        request.get().set_n(234);
        let server = impls::TestInterface::new();
        let chained_call_count = server.get_call_count();
        request.get().set_in_cap(
            crate::test_capnp::test_interface::ToClient::new(server).into_client::<::capnp_rpc::Server>());

        let promise = request.send();

        let mut pipeline_request = promise.pipeline.get_out_box().get_cap().foo_request();
        pipeline_request.get().set_i(321);
        let pipeline_promise = pipeline_request.send();

        let pipeline_request2 = {
            let extends_client =
                crate::test_capnp::test_extends::Client { client: promise.pipeline.get_out_box().get_cap().client };
            extends_client.grault_request()
        };
        let pipeline_promise2 = pipeline_request2.send();

        drop(promise); // Just to be annoying, drop the original promise.

        if chained_call_count.get() != 0 {
            return Err(Error::failed("expected chained_call_count to equal 0".to_string()));
        }

        let response = exec.run_until(pipeline_promise.promise)?;

        if response.get()?.get_x()? != "bar" {
            return Err(Error::failed("expected x to equal 'bar'".to_string()));
        }

        let response2 = exec.run_until(pipeline_promise2.promise)?;
        crate::test_util::CheckTestMessage::check_test_message(response2.get()?);
        assert_eq!(chained_call_count.get(), 1);
        Ok(())
    });
}

#[test]
fn pipelining_return_null() {
    rpc_top_level(|mut exec, client| {
        let response = exec.run_until(client.test_pipeline_request().send().promise)?;
        let client = response.get()?.get_cap()?;

        let request = client.get_null_cap_request();
        let cap = request.send().pipeline.get_cap();
        match exec.run_until(cap.foo_request().send().promise) {
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
    let root: crate::test_capnp::test_all_types::Builder = message.get_root().unwrap();

    // In capnproto-c++, this would return a BrokenCap. Here, it returns a decode error.
    // Would it be worthwhile to try to match the C++ behavior here? We would need something
    // like the BrokenCapFactory singleton.
    assert!(root.get_interface_field().is_err());
}

#[test]
fn release_simple() {
    rpc_top_level(|mut exec, client| {
        let response = exec.run_until(client.test_more_stuff_request().send().promise)?;
        let client = response.get()?.get_cap()?;

        let handle1 = client.get_handle_request().send().promise;
        let ::capnp::capability::RemotePromise {promise, pipeline} = client.get_handle_request().send();
        let handle2 = exec.run_until(promise)?.get()?.get_handle()?;

        let get_count_response = exec.run_until(client.get_handle_count_request().send().promise)?;
        if get_count_response.get()?.get_count() != 2 {
            return Err(Error::failed("expected handle count to equal 2".to_string()))
        }

        drop(handle1);

        let get_count_response = exec.run_until(client.get_handle_count_request().send().promise)?;
        if get_count_response.get()?.get_count() != 1 {
            return Err(Error::failed("expected handle count to equal 1".to_string()))
        }

        drop(handle2);

        let get_count_response = exec.run_until(client.get_handle_count_request().send().promise)?;
        if get_count_response.get()?.get_count() != 1 {
            return Err(Error::failed("expected handle count to equal 1".to_string()))
        }

        drop(pipeline);

        let get_count_response = exec.run_until(client.get_handle_count_request().send().promise)?;
        if get_count_response.get()?.get_count() != 0 {
            return Err(Error::failed("expected handle count to equal 0".to_string()))
        }

        Ok(())
    });
}

/*
#[test]
fn release_on_cancel() {
    rpc_top_level(|mut exec, client| {
        let response = exec.run_until(client.test_more_stuff_request().send().promise)?;
        let client = response.get()?.get_cap()?;

        let promise = client.get_handle_request().send();

        // If the server receives cancellation too early, it won't even return a capability in the
        // results, it will just return "canceled". We want to emulate the case where the return message
        // and the cancel (finish) message cross paths.

        // TODO Verify that this is actually testing what we're interested in.

        core.turn(Some(::std::time::Duration::from_millis(1)));
        core.turn(Some(::std::time::Duration::from_millis(1)));

        drop(promise);

        for _ in 0..16 {
            core.turn(Some(::std::time::Duration::from_millis(1)));
        }

        let get_count_response = core.run(client.get_handle_count_request().send().promise)?;
        let handle_count = get_count_response.get()?.get_count();
        if handle_count != 0 {
            return Err(Error::failed(format!("handle count: expected 0, but got {}", handle_count)))
        }
        Ok(())
    });
}
*/

#[test]
fn promise_resolve() {
    rpc_top_level(|mut exec, client| {
        let response = exec.run_until(client.test_more_stuff_request().send().promise)?;
        let client = response.get()?.get_cap()?;

        let mut request = client.call_foo_request();
        let mut request2 = client.call_foo_when_resolved_request();

        let (paf_fulfiller, paf_promise) = oneshot::channel();
        let cap: crate::test_capnp::test_interface::Client =
            ::capnp_rpc::new_promise_client(paf_promise.map_err(canceled_to_error));
        request.get().set_cap(cap.clone());
        request2.get().set_cap(cap);

        let promise = request.send().promise;
        let promise2 = request2.send().promise;

        // Make sure getCap() has been called on the server side by sending another call and waiting
        // for it.
        let client2 = crate::test_capnp::test_call_order::Client { client: client.clone().client };
        let _response = exec.run_until(client2.get_call_sequence_request().send().promise)?;

        let server = impls::TestInterface::new();
        let _ = paf_fulfiller.send(
            crate::test_capnp::test_interface::ToClient::new(server).into_client::<::capnp_rpc::Server>().client);

        let response = exec.run_until(promise)?;
        if response.get()?.get_s()? != "bar" {
            return Err(Error::failed("expected s to equal 'bar'".to_string()));
        }
        let response = exec.run_until(promise2)?;
        if response.get()?.get_s()? != "bar" {
            return Err(Error::failed("expected s to equal 'bar'".to_string()));
        }
        Ok(())
    });
}

#[test]
fn retain_and_release() {
    use std::cell::Cell;
    use std::rc::Rc;

    rpc_top_level(|mut exec, client| {
        let (fulfiller, promise) = oneshot::channel::<()>();
        let destroyed = Rc::new(Cell::new(false));

        let (destroyed_done_sender, destroyed_done_receiver) = oneshot::channel::<()>();

        let destroyed1 = destroyed.clone();
        let mut spawner = exec.spawner();
        spawn(&mut spawner, promise.map_err(canceled_to_error).map(move |r| {
            r?;
            destroyed1.set(true);
            let _ = destroyed_done_sender.send(());
            Ok(())
        }));

        {
            let response = exec.run_until(client.test_more_stuff_request().send().promise)?;
            let client = response.get()?.get_cap()?;

            {
                let mut request = client.hold_request();
                request.get().set_cap(
                    crate::test_capnp::test_interface::ToClient::new(impls::TestCapDestructor::new(fulfiller))
                        .into_client::<::capnp_rpc::Server>());
                exec.run_until(request.send().promise)?;
            }

            // Do some other call to add a round trip.
            // ugh, we need upcasting.
            let client1 = crate::test_capnp::test_call_order::Client { client: client.clone().client };
            let response = exec.run_until(client1.get_call_sequence_request().send().promise)?;
            if response.get()?.get_n() != 1 {
                return Err(Error::failed("N should equal 1".to_string()))
            }

            if destroyed.get() {
                return Err(Error::failed("shouldn't be destroyed yet".to_string()))
            }


            // We can ask it to call the held capability.
            let response = exec.run_until(client.call_held_request().send().promise)?;
            if response.get()?.get_s()? != "bar" {
                return Err(Error::failed("S should equal 'bar'".to_string()))
            }

            {
                // we can get the cap back from it.
                let response = exec.run_until(client.get_held_request().send().promise)?;
                let cap_copy = response.get()?.get_cap()?;

                // And call it, without any network communications.
                // (TODO: verify that no network communications happen here)
                {
                    let mut request = cap_copy.foo_request();
                    request.get().set_i(123);
                    request.get().set_j(true);
                    let response = exec.run_until(request.send().promise)?;
                    if response.get()?.get_x()? != "foo" {
                        return Err(Error::failed("X should equal 'foo'.".to_string()));
                    }
                }

                {
                    // We can send another copy of the same cap to another method, and it works.
                    let mut request = client.call_foo_request();
                    request.get().set_cap(cap_copy);
                    let response = exec.run_until(request.send().promise)?;
                    if response.get()?.get_s()? != "bar" {
                        return Err(Error::failed("S should equal 'bar'.".to_string()));
                    }
                }
            }

            // Give some time to settle.
            let response = exec.run_until(client1.get_call_sequence_request().send().promise)?;
            if response.get()?.get_n() != 5 {
                return Err(Error::failed("N should equal 5.".to_string()));
            }
            let response = exec.run_until(client1.get_call_sequence_request().send().promise)?;
            if response.get()?.get_n() != 6 {
                return Err(Error::failed("N should equal 6.".to_string()));
            }
            let response = exec.run_until(client1.get_call_sequence_request().send().promise)?;
            if response.get()?.get_n() != 7 {
                return Err(Error::failed("N should equal 7.".to_string()));
            }

            if destroyed.get() {
                return Err(Error::failed("haven't released it yet".to_string()))
            }
        }

        exec.run_until(destroyed_done_receiver.map_err(canceled_to_error))?;
        if !destroyed.get() {
            return Err(Error::failed("should be destroyed now".to_string()));
        }

        Ok(())
    });
}

#[test]
fn cancel_releases_params() {
    use std::rc::Rc;
    use std::cell::Cell;

    rpc_top_level(|mut exec, client| {
        let response = exec.run_until(client.test_more_stuff_request().send().promise)?;
        let client = response.get()?.get_cap()?;

        let (fulfiller, promise) = oneshot::channel::<()>();
        let destroyed = Rc::new(Cell::new(false));

        let (destroyed_done_sender, destroyed_done_receiver) = oneshot::channel::<()>();

        let destroyed1 = destroyed.clone();
        let mut spawner = exec.spawner();
        spawn(&mut spawner,
              promise.map_err(canceled_to_error).map(move |r| {
                  r?;
                  destroyed1.set(true);
                  let _ = destroyed_done_sender.send(());
                  Ok(())
              }));

        {
            let mut request = client.never_return_request();
            request.get().set_cap(
                crate::test_capnp::test_interface::ToClient::new(impls::TestCapDestructor::new(fulfiller))
                    .into_client::<::capnp_rpc::Server>());

            {
                let _response_promise = request.send();

                // Allow some time to settle.

                // ugh, we need upcasting.
                let client = crate::test_capnp::test_call_order::Client { client: client.client };
                let response = exec.run_until(client.get_call_sequence_request().send().promise)?;
                if response.get()?.get_n() != 1 {
                    return Err(Error::failed("N should equal 1.".to_string()));
                }
                let response = exec.run_until(client.get_call_sequence_request().send().promise)?;
                if response.get()?.get_n() != 2 {
                    return Err(Error::failed("N should equal 2.".to_string()));
                }
                if destroyed.get() {
                    return Err(Error::failed("Shouldn't be destroyed yet.".to_string()));
                }
            }
        }

        exec.run_until(destroyed_done_receiver.map_err(canceled_to_error))?;
        if !destroyed.get() {
            return Err(Error::failed("The cap should be released now.".to_string()));
        }

        Ok(())
    });
}

#[test]
fn dont_hold() {
    rpc_top_level(|mut exec, client| {
        let response = exec.run_until(client.test_more_stuff_request().send().promise)?;
        let client = response.get()?.get_cap()?;

        let (fulfiller, promise) = oneshot::channel();
        let cap: crate::test_capnp::test_interface::Client =
            ::capnp_rpc::new_promise_client(promise.map_err(canceled_to_error));

        let mut request = client.dont_hold_request();
        request.get().set_cap(cap.clone());

        exec.run_until(request.send().promise.and_then(move |_response| {
            let mut request = client.dont_hold_request();
            request.get().set_cap(cap.clone());
            request.send().promise.and_then(move |_| {
                drop(fulfiller);
                Promise::ok(())
            })
        }))
    });
}


fn get_call_sequence(client: &crate::test_capnp::test_call_order::Client, expected: u32)
                     -> ::capnp::capability::RemotePromise<crate::test_capnp::test_call_order::get_call_sequence_results::Owned>
{
    let mut req = client.get_call_sequence_request();
    req.get().set_expected(expected);
    req.send()
}


#[test]
fn embargo_success() {
    rpc_top_level(|mut exec, client| {
        let response = exec.run_until(client.test_more_stuff_request().send().promise)?;
        let client = response.get()?.get_cap()?;

        let server = crate::impls::TestCallOrder::new();

        // ugh, we need upcasting.
        let client2 = crate::test_capnp::test_call_order::Client { client: client.clone().client };
        let early_call = client2.get_call_sequence_request().send();
        drop(client2);

        let mut echo_request = client.echo_request();
        echo_request.get().set_cap(
            crate::test_capnp::test_call_order::ToClient::new(server).into_client::<::capnp_rpc::Server>());
        let echo = echo_request.send();

        let pipeline = echo.pipeline.get_cap();

        let call0 = get_call_sequence(&pipeline, 0);
        let call1 = get_call_sequence(&pipeline, 1);

        exec.run_until(early_call.promise)?;

        let call2 = get_call_sequence(&pipeline, 2);

        let _resolved = exec.run_until(echo.promise)?;

        let call3 = get_call_sequence(&pipeline, 3);
        let call4 = get_call_sequence(&pipeline, 4);
        let call5 = get_call_sequence(&pipeline, 5);

        exec.run_until(futures::future::try_join_all(
            vec![call0.promise,
                 call1.promise,
                 call2.promise,
                 call3.promise,
                 call4.promise,
                 call5.promise
            ]).map(|responses| {
                let mut counter = 0;
                for r in responses?.into_iter() {
                    if counter != r.get()?.get_n() {
                        return Err(Error::failed(
                            "calls arrived out of order".to_string()))
                    }
                    counter += 1;
                }
                Ok(())
        }))
    });
}



fn expect_promise_throws<T>(promise: Promise<T, Error>, exec: &mut futures::executor::LocalPool)
                            -> Result<(), Error> {
    exec.run_until(promise.map(|r| match r {
        Ok(_) => Err(Error::failed("expected promise to fail".to_string())),
        Err(_) => Ok(()),
    }))
}

#[test]
fn embargo_error() {
    rpc_top_level(|mut exec, client| {
        let response = exec.run_until(client.test_more_stuff_request().send().promise)?;
        let client = response.get()?.get_cap()?;

        let (fulfiller, promise) = oneshot::channel();
        let cap: crate::test_capnp::test_call_order::Client =
            ::capnp_rpc::new_promise_client(promise.map_err(canceled_to_error));

        // ugh, we need upcasting.
        let client2 = crate::test_capnp::test_call_order::Client { client: client.clone().client };
        let early_call = client2.get_call_sequence_request().send();
        drop(client2);

        let mut echo_request = client.echo_request();
        echo_request.get().set_cap(cap);
        let echo = echo_request.send();

        let pipeline = echo.pipeline.get_cap();

        let call0 = get_call_sequence(&pipeline, 0);
        let call1 = get_call_sequence(&pipeline, 1);

        exec.run_until(early_call.promise)?;

        let call2 = get_call_sequence(&pipeline, 2);

        let _resolved = exec.run_until(echo.promise);

        let call3 = get_call_sequence(&pipeline, 3);
        let call4 = get_call_sequence(&pipeline, 4);
        let call5 = get_call_sequence(&pipeline, 5);

        drop(fulfiller);

        expect_promise_throws(call0.promise, &mut exec)?;
        expect_promise_throws(call1.promise, &mut exec)?;
        expect_promise_throws(call2.promise, &mut exec)?;
        expect_promise_throws(call3.promise, &mut exec)?;
        expect_promise_throws(call4.promise, &mut exec)?;
        expect_promise_throws(call5.promise, &mut exec)?;
        Ok(())
    });
}

#[test]
fn echo_destruction() {
    rpc_top_level(|mut exec, client| {
        let response = exec.run_until(client.test_more_stuff_request().send().promise)?;
        let client = response.get()?.get_cap()?;

        let (fulfiller, promise) = oneshot::channel();
        let cap: crate::test_capnp::test_call_order::Client =
            ::capnp_rpc::new_promise_client(promise.map_err(canceled_to_error));

        // ugh, we need upcasting.
        let client2 = crate::test_capnp::test_call_order::Client { client: client.clone().client };
        let early_call = client2.get_call_sequence_request().send();
        drop(client2);

        let mut echo_request = client.echo_request();
        echo_request.get().set_cap(cap);
        let echo = echo_request.send();

        let pipeline = echo.pipeline.get_cap();

        exec.run_until(early_call.promise.and_then(move |_early_call_response| {
            let _ = get_call_sequence(&pipeline, 2);
            echo.promise.and_then(move |_echo_response| {
                drop(fulfiller);
                Promise::ok(())
            })
        }))
    })
}

#[test]
fn local_client_call_not_immediate() {
    let server = crate::impls::TestInterface::new();
    let call_count = server.get_call_count();
    assert_eq!(call_count.get(), 0);
    let client = crate::test_capnp::test_interface::ToClient::new(server).into_client::<::capnp_rpc::Server>();
    let mut req = client.foo_request();
    req.get().set_i(123);
    req.get().set_j(true);
    let remote_promise = req.send();

    // Hm... do we actually care about this?
    assert_eq!(call_count.get(), 0);

    let mut exec = futures::executor::LocalPool::new();
    let _ = exec.run_until(remote_promise.promise);
    assert_eq!(call_count.get(), 1);
}

#[test]
fn local_client_send_cap() {
    let server1 = crate::impls::TestMoreStuff::new();
    let server2 = crate::impls::TestInterface::new();
    let client1 = crate::test_capnp::test_more_stuff::ToClient::new(server1).into_client::<::capnp_rpc::Server>();
    let client2 = crate::test_capnp::test_interface::ToClient::new(server2).into_client::<::capnp_rpc::Server>();

    let mut req = client1.call_foo_request();
    req.get().set_cap(client2);
    let mut exec = futures::executor::LocalPool::new();
    let response = exec.run_until(req.send().promise).unwrap();
    assert_eq!(response.get().unwrap().get_s().unwrap(), "bar");
}

#[test]
fn local_client_return_cap() {
    let server = crate::impls::Bootstrap;
    let client = crate::test_capnp::bootstrap::ToClient::new(server).into_client::<::capnp_rpc::Server>();
    let mut exec = futures::executor::LocalPool::new();
    let response = exec.run_until(client.test_interface_request().send().promise).unwrap();
    let client1 = response.get().unwrap().get_cap().unwrap();

    let mut request = client1.foo_request();
    request.get().set_i(123);
    request.get().set_j(true);
    let response1 = exec.run_until(request.send().promise).unwrap();
    assert_eq!(response1.get().unwrap().get_x().unwrap(), "foo");
}

#[test]
fn capability_list() {
    rpc_top_level(|mut exec, client| {
        let response = exec.run_until(client.test_more_stuff_request().send().promise)?;
        let client = response.get()?.get_cap()?;

        let server1 = crate::impls::TestInterface::new();
        let call_count1 = server1.get_call_count();
        assert_eq!(call_count1.get(), 0);
        let client1 = crate::test_capnp::test_interface::ToClient::new(server1).into_client::<::capnp_rpc::Server>();

        let server2 = crate::impls::TestInterface::new();
        let call_count2 = server2.get_call_count();
        assert_eq!(call_count2.get(), 0);
        let client2 = crate::test_capnp::test_interface::ToClient::new(server2).into_client::<::capnp_rpc::Server>();

        let mut request = client.call_each_capability_request();
        {
            let mut caps = request.get().init_caps(2);
            caps.set(0, client1.client.hook);
            caps.set(1, client2.client.hook);
        }
        exec.run_until(request.send().promise)?;
        assert_eq!(call_count1.get(), 1);
        assert_eq!(call_count2.get(), 1);
        Ok(())
    })
}
