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

#[macro_use]
extern crate gj;

use gj::{EventLoop, Promise};
use gj::io::{AsyncRead, AsyncWrite, unix};
use capnp::Error;
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp, twoparty};

pub mod test_capnp {
  include!(concat!(env!("OUT_DIR"), "/test_capnp.rs"));
}

pub mod impls;
pub mod test_util;


struct ReadWrapper<R> where R: AsyncRead {
    inner: R,
    output_file: ::std::fs::File,
}

impl <R> ReadWrapper<R> where R: AsyncRead {
    fn new(inner: R, output_file: ::std::fs::File) -> ReadWrapper<R> {
        ReadWrapper {
            inner: inner,
            output_file: output_file,
        }
    }
}

impl <R> AsyncRead for ReadWrapper<R> where R: AsyncRead {
    fn try_read<T>(self, buf: T, min_bytes: usize)
                   -> Promise<(ReadWrapper<R>, T, usize), ::gj::io::Error<(ReadWrapper<R>, T)>>
        where T: AsMut<[u8]>
    {
        let ReadWrapper {inner, mut output_file} = self;
        inner.try_read(buf, min_bytes).map_else(|r| match r {
            Ok((r, mut buf, n)) => {
                use std::io::Write;
                output_file.write(&buf.as_mut()[0..n]).unwrap();
                Ok((ReadWrapper::new(r, output_file), buf, n))
            }
            Err(::gj::io::Error { state: (r, buf), error }) =>
                Err(::gj::io::Error::new((ReadWrapper::new(r, output_file), buf), error)),
        })
    }
}

struct WriteWrapper<W> where W: AsyncWrite {
    inner: W,
    output_file: ::std::fs::File,
}

impl <W> WriteWrapper<W> where W: AsyncWrite {
    fn new(inner: W, output_file: ::std::fs::File) -> WriteWrapper<W> {
        WriteWrapper {
            inner: inner,
            output_file: output_file,
        }
    }
}

impl <W> AsyncWrite for WriteWrapper<W> where W: AsyncWrite {
    fn write<T>(self, buf: T)
                -> Promise<(WriteWrapper<W>, T), ::gj::io::Error<(WriteWrapper<W>, T)>>
        where T: AsRef<[u8]>
    {
        use std::io::Write;
        let WriteWrapper {inner, mut output_file} = self;
        output_file.write(buf.as_ref()).unwrap();

        inner.write(buf).map_else(|r| match r {
            Ok((w, buf)) => {
                Ok((WriteWrapper::new(w, output_file), buf))
            }
            Err(::gj::io::Error { state: (w, buf), error }) =>
                Err(::gj::io::Error::new((WriteWrapper::new(w, output_file), buf), error)),
        })
    }
}


#[test]
fn drop_rpc_system() {
    EventLoop::top_level(|wait_scope| {
        let (instream, _outstream) = try!(unix::Stream::new_pair());
        let (reader, writer) = instream.split();
        let network =
            Box::new(twoparty::VatNetwork::new(reader, writer,
                                               rpc_twoparty_capnp::Side::Client,
                                               Default::default()));
        let rpc_system = RpcSystem::new(network, None);
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
            //let reader = ReadWrapper::new(reader,
            //                              ::std::fs::File::create("/Users/dwrensha/Desktop/test1.dat").unwrap());
            let mut network =
                Box::new(twoparty::VatNetwork::new(reader, writer,
                                                   rpc_twoparty_capnp::Side::Server,
                                                   Default::default()));
            let disconnect_promise = network.on_disconnect();
            let bootstrap =
                test_capnp::bootstrap::ToClient::new(impls::Bootstrap).from_server::<::capnp_rpc::Server>();

            let _rpc_system = RpcSystem::new(network, Some(bootstrap.client));
            try!(disconnect_promise.wait(wait_scope));
            Ok(())
        }));

        let (reader, writer) = stream.split();
        //let writer = WriteWrapper::new(writer,
        //                               ::std::fs::File::create("/Users/dwrensha/Desktop/test2.dat").unwrap());

        let network =
            Box::new(twoparty::VatNetwork::new(reader, writer,
                                               rpc_twoparty_capnp::Side::Client,
                                               Default::default()));

        let mut rpc_system = RpcSystem::new(network, None);
        let client: test_capnp::bootstrap::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

        try!(main(client).wait(wait_scope));
        drop(rpc_system);
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

            let mut request2 = client.baz_request();
            ::test_util::init_test_message(pry!(request2.get().get_s()));
            let promise2 = request2.send().promise.map(|_| Ok(()));

            Promise::all(vec![promise1, promise2, promise3].into_iter()).map(|_| Ok(()))
        })
    });
}

#[test]
fn pipelining() {
    set_up_rpc(|client| {
        client.test_pipeline_request().send().promise.then(|response| {
            let client = pry!(pry!(response.get()).get_cap());
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
                return Promise::err(Error::failed("expeced chained_call_count to equal 0".to_string()));
            }

            pipeline_promise.promise.then(move |response| {
                if pry!(pry!(response.get()).get_x()) != "bar" {
                    return Promise::err(Error::failed("expected x to equal 'bar'".to_string()));
                }
                pipeline_promise2.promise.then(move |response| {
                    ::test_util::CheckTestMessage::check_test_message(pry!(response.get()));
                    assert_eq!(chained_call_count.get(), 1);
                    Promise::ok(())
                })
            })
        })
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
    set_up_rpc(|client| {
        client.test_more_stuff_request().send().promise.then(|response| {
            let client = pry!(pry!(response.get()).get_cap());

            Promise::all(vec![
                client.get_handle_request().send().promise,
                client.get_handle_request().send().promise].into_iter()).then(move |responses| {
                let handle1 = pry!(pry!(responses[0].get()).get_handle());
                let handle2 = pry!(pry!(responses[1].get()).get_handle());

                let client1 = client.clone();
                let client2 = client.clone();
                client.get_handle_count_request().send().promise.map(|response| {
                    if try!(response.get()).get_count() != 2 {
                        Err(Error::failed("expected handle count to equal 2".to_string()))
                    } else {
                        Ok(())
                    }
                }).then(move |()| {
                    drop(handle1);
                    client1.get_handle_count_request().send().promise.map(|response| {
                        if try!(response.get()).get_count() != 1 {
                            Err(Error::failed("expected handle count to equal 1".to_string()))
                        } else {
                            Ok(())
                        }
                    })
                }).then(move |()| {
                    drop(handle2);
                    client2.get_handle_count_request().send().promise.map(|response| {
                        if try!(response.get()).get_count() != 0 {
                            Err(Error::failed("expected handle count to equal 0".to_string()))
                        } else {
                            Ok(())
                        }
                    })
                })
            })
        })
    });
}

#[test]
fn promise_resolve() {
    set_up_rpc(|client| {
        client.test_more_stuff_request().send().promise.then(|response| {
            let client = pry!(pry!(response.get()).get_cap());

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
            client2.get_call_sequence_request().send().promise.then(move |_response| {
                let server = impls::TestInterface::new();

                paf_fulfiller.fulfill(
                    ::test_capnp::test_interface::ToClient::new(server).from_server::<::capnp_rpc::Server>());

                promise.then(move |response| {
                    if pry!(pry!(response.get()).get_s()) != "bar" {
                        return Promise::err(Error::failed("expected s to equal 'bar'".to_string()));
                    }
                    promise2.then(move |response| {
                        if pry!(pry!(response.get()).get_s()) != "bar" {
                            return Promise::err(Error::failed("expected s to equal 'bar'".to_string()));
                        }
                        Promise::ok(())
                    })
                })
            })
        })
    });
}

#[test]
fn retain_and_release() {
    use std::cell::Cell;
    use std::rc::Rc;

    set_up_rpc(|client| {
        client.test_more_stuff_request().send().promise.then(|response| {
            let client = pry!(pry!(response.get()).get_cap());

            let (promise, fulfiller) = Promise::<(), Error>::and_fulfiller();
            let destroyed = Rc::new(Cell::new(false));

            let destroyed1 = destroyed.clone();
            let destruction_promise = promise.map(move |()| {
                destroyed1.set(true);
                Ok(())
            }).eagerly_evaluate();

            let client1 = client.clone();
            let client2 = client.clone();
            let client3 = client.clone();
            let client4 = client.clone();
            let destroyed2 = destroyed.clone();
            let destroyed3 = destroyed.clone();
            let mut request = client.hold_request();
            request.get().set_cap(
                ::test_capnp::test_interface::ToClient::new(impls::TestCapDestructor::new(fulfiller))
                    .from_server::<::capnp_rpc::Server>());
            request.send().promise.then(move |_response| {
                // Do some other call to add a round trip.

                // ugh, we need upcasting.
                let client1 = ::test_capnp::test_call_order::Client { client: client1.client };
                client1.get_call_sequence_request().send().promise
            }).then(move |response| {
                if pry!(response.get()).get_n() != 1 {
                    Promise::err(Error::failed("N should equal 1".to_string()))
                } else if destroyed.get() {
                    Promise::err(Error::failed("shouldn't be destroyed yet".to_string()))
                } else {
                    // We can ask it to call the held capability.
                    client2.call_held_request().send().promise.map(|response| {
                        if try!(try!(response.get()).get_s()) != "bar" {
                            Err(Error::failed("S should equal 'bar'".to_string()))
                        } else {
                            Ok(())
                        }
                    })
                }
            }).then(move |()| {
                // we can get the cap back from it.
                client3.get_held_request().send().promise.map(move |response| {
                    try!(response.get()).get_cap()
                })
            }).then(move |cap_copy| {
                // And call it, without any network communications.
                // (TODO: verify that no network communications happen here)
                let mut request = cap_copy.foo_request();
                request.get().set_i(123);
                request.get().set_j(true);
                request.send().promise.map(|response| {
                    if try!(try!(response.get()).get_x()) != "foo" {
                        Err(Error::failed("X should equal 'foo'.".to_string()))
                    } else {
                        Ok(cap_copy)
                    }
                })
            }).then(move |cap_copy| {
                // We can send another copy of the same cap to another method, and it works.
                let mut request = client4.call_foo_request();
                request.get().set_cap(cap_copy);
                request.send().promise.map(|response| {
                    if try!(try!(response.get()).get_s()) != "bar" {
                        Err(Error::failed("S should equal 'bar'.".to_string()))
                    } else {
                        Ok(())
                    }
                })
            }).then(move |()| {
                // Give some time to settle.

                // ugh, we need upcasting.
                let client = ::test_capnp::test_call_order::Client { client: client.client };
                client.get_call_sequence_request().send().promise.map(move |response| {
                    if try!(response.get()).get_n() != 5 {
                        Err(Error::failed("N should equal 5.".to_string()))
                    } else {
                        Ok(client)
                    }
                })
            }).then(|client| {
                client.get_call_sequence_request().send().promise.map(move |response| {
                    if try!(response.get()).get_n() != 6 {
                        Err(Error::failed("N should equal 6.".to_string()))
                    } else {
                        Ok(client)
                    }
                })
            }).then(|client| {
                client.get_call_sequence_request().send().promise.map(move |response| {
                    if try!(response.get()).get_n() != 7 {
                        Err(Error::failed("N should equal 7.".to_string()))
                    } else {
                        Ok(client)
                    }
                })
            }).then(move |_client| {
                if destroyed2.get() {
                    Promise::err(Error::failed("haven't released it yet".to_string()))
                } else {
                    Promise::ok(())
                }
            }).then(|()| {
                // There are no references to `client` left.
                destruction_promise
            }).map(move |()| {
                if destroyed3.get() {
                    Ok(())
                } else {
                    Err(Error::failed("should be destroyed now".to_string()))
                }
            })
        })
    });
}


#[test]
fn dont_hold() {
    set_up_rpc(|client| {
        client.test_more_stuff_request().send().promise.then(|response| {
            let client = pry!(pry!(response.get()).get_cap());

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
            })
        })
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
    set_up_rpc(|client| {
        client.test_more_stuff_request().send().promise.then(|response| {
            let client = pry!(pry!(response.get()).get_cap());

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

            early_call.promise.then(move |_early_call_response| {
                let call2 = get_call_sequence(&pipeline, 2);
                echo.promise.then(move |_echo_response| {
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
                    })
                })
            })
        })
    });
}

fn expect_promise_throws<T>(promise: Promise<T, Error>) -> Promise<(), Error> {
    promise.then_else(|r| match r {
        Ok(_) => Promise::err(Error::failed("expected promise to fail".to_string())),
        Err(_) => {
            Promise::ok(())
        }
    })
}

#[test]
fn embargo_error() {
    set_up_rpc(|client| {
        client.test_more_stuff_request().send().promise.then(|response| {
            let client = pry!(pry!(response.get()).get_cap());

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

            early_call.promise.then(move |_early_call_response| {
                let call2 = get_call_sequence(&pipeline, 2);
                echo.promise.then(move |_echo_response| {
                    let call3 = get_call_sequence(&pipeline, 3);
                    let call4 = get_call_sequence(&pipeline, 4);
                    let call5 = get_call_sequence(&pipeline, 5);
                    fulfiller.reject(Error::failed("foo".to_string()));
                    Promise::all(vec![
                        expect_promise_throws(call0.promise),
                        expect_promise_throws(call1.promise),
                        expect_promise_throws(call2.promise),
                        expect_promise_throws(call3.promise),
                        expect_promise_throws(call4.promise),
                        expect_promise_throws(call5.promise)].into_iter()).map(|_| Ok(()))
                })
            })
        })
    });
}

#[test]
fn echo_destruction() {
    set_up_rpc(|client| {
        client.test_more_stuff_request().send().promise.then(|response| {
            let client = pry!(pry!(response.get()).get_cap());

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
            })
        })
    });
}
