// Copyright (c) 2015 Sandstorm Development Group, Inc. and contributors
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

//! An implementation of `VatNetwork` for the common case of a client-server connection.

use capnp::message::ReaderOptions;
use futures::Future;
use futures::sync::oneshot;

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use {Promise, ForkedPromise};

pub type VatId = ::rpc_twoparty_capnp::Side;

struct IncomingMessage {
    message: ::capnp::message::Reader<::capnp_futures::serialize::OwnedSegments>,
}

impl IncomingMessage {
    pub fn new(message: ::capnp::message::Reader<::capnp_futures::serialize::OwnedSegments>) -> IncomingMessage {
        IncomingMessage { message: message }
    }
}

impl ::IncomingMessage for IncomingMessage {
    fn get_body<'a>(&'a self) -> ::capnp::Result<::capnp::any_pointer::Reader<'a>> {
        self.message.get_root()
    }
}

struct OutgoingMessage<U> where U: ::std::io::Write + 'static {
    message: ::capnp::message::Builder<::capnp::message::HeapAllocator>,
    write_queue: Rc<RefCell<Promise<U, ::capnp::Error>>>,
}

impl <U> ::OutgoingMessage for OutgoingMessage<U> where U: ::std::io::Write {
    fn get_body<'a>(&'a mut self) -> ::capnp::Result<::capnp::any_pointer::Builder<'a>> {
        self.message.get_root()
    }

    fn get_body_as_reader<'a>(&'a self) -> ::capnp::Result<::capnp::any_pointer::Reader<'a>> {
        self.message.get_root_as_reader()
    }

    fn send(self: Box<Self>)
            -> Promise<::capnp::message::Builder<::capnp::message::HeapAllocator>, ::capnp::Error>
    {
        let tmp = *self;
        let OutgoingMessage {message, write_queue} = tmp;
        let queue = ::std::mem::replace(&mut *write_queue.borrow_mut(), Promise::never_done());
        let (fulfiller, promise) = oneshot::channel();
        *write_queue.borrow_mut() = queue.then(move |s| {
            ::capnp_futures::serialize::write_message(s, message).map(move |(s, m)| {
                fulfiller.fulfill(m);
                Ok(s)
            }).eagerly_evaluate()
        });
        promise
    }

    fn take(self: Box<Self>)
            -> ::capnp::message::Builder<::capnp::message::HeapAllocator>
    {
        self.message
    }
}

struct ConnectionInner<T, U> where T: ::std::io::Read + 'static, U: ::std::io::Write + 'static {
    input_stream: Rc<RefCell<Option<T>>>,
    write_queue: Rc<RefCell<Promise<U, ::capnp::Error>>>,
    side: ::rpc_twoparty_capnp::Side,
    receive_options: ReaderOptions,
    on_disconnect_fulfiller: Option<oneshot::Sender<()>>,
}

struct Connection<T, U> where T: ::std::io::Read + 'static, U: ::std::io::Write + 'static {
    inner: Rc<RefCell<ConnectionInner<T, U>>>,
}

impl <T, U> Drop for ConnectionInner<T, U> where T: ::std::io::Read, U: ::std::io::Write {
    fn drop(&mut self) {
        let maybe_fulfiller = ::std::mem::replace(&mut self.on_disconnect_fulfiller, None);
        match maybe_fulfiller {
            Some(fulfiller) => {
                fulfiller.fulfill(());
            }
            None => unreachable!(),
        }
    }
}

impl <T, U> Connection<T, U> where T: ::std::io::Read, U: ::std::io::Write {
    fn new(input_stream: T,
           output_stream: U,
           side: ::rpc_twoparty_capnp::Side,
           receive_options: ReaderOptions,
           on_disconnect_fulfiller: oneshot::Sender<()>,
           ) -> Connection<T, U> {
        Connection {
            inner: Rc::new(RefCell::new(
                ConnectionInner {
                    input_stream: Rc::new(RefCell::new(Some(input_stream))),
                    write_queue: Rc::new(RefCell::new(::gj::Promise::ok(output_stream))),
                    side: side,
                    receive_options: receive_options,
                    on_disconnect_fulfiller: Some(on_disconnect_fulfiller),
                })),
        }
    }
}

impl <T, U> ::Connection<::rpc_twoparty_capnp::Side> for Connection<T, U>
    where T: ::std::io::Read, U: ::std::io::Write
{
    fn get_peer_vat_id(&self) -> ::rpc_twoparty_capnp::Side {
        self.inner.borrow().side
    }

    fn new_outgoing_message(&mut self, _first_segment_word_size: u32) -> Box<::OutgoingMessage> {
        Box::new(OutgoingMessage {
            message: ::capnp::message::Builder::new_default(),
            write_queue: self.inner.borrow().write_queue.clone()
        })
    }

    fn receive_incoming_message(&mut self) -> Promise<Option<Box<::IncomingMessage>>, ::capnp::Error> {
        let mut inner = self.inner.borrow_mut();
        let maybe_input_stream = ::std::mem::replace(&mut *inner.input_stream.borrow_mut(), None);
        let return_it_here = inner.input_stream.clone();
        match maybe_input_stream {
            Some(s) => {
                ::capnp_futures::serialize::read_message(s, inner.receive_options).map(move |(s, maybe_message)| {
                    *return_it_here.borrow_mut() = Some(s);
                    Ok(maybe_message.map(|message|
                                         Box::new(IncomingMessage::new(message)) as Box<::IncomingMessage>))
                })
            }
            None => unreachable!(),
        }
    }

    fn shutdown(&mut self) -> Promise<(), ::capnp::Error> {
        let mut inner = self.inner.borrow_mut();
        let write_queue = ::std::mem::replace(&mut *inner.write_queue.borrow_mut(), Promise::never_done());
        write_queue.map(|_| Ok(()))
    }
}

/// A vat networks with two parties, the client and the server.
pub struct VatNetwork<T, U> where T: ::std::io::Read + 'static, U: ::std::io::Write + 'static {
    connection: Option<Connection<T, U>>,

    // HACK
    weak_connection_inner: Weak<RefCell<ConnectionInner<T, U>>>,

    on_disconnect_promise: ForkedPromise<Promise<(), ::capnp::Error>>,
    side: ::rpc_twoparty_capnp::Side,
}

impl <T, U> VatNetwork<T, U> where T: ::std::io::Read, U: ::std::io::Write {
    /// Creates a new two-party vat network that will receive data on `input_stream` and send data on
    /// `output_stream`. `side` indicates whether this is the client or the server side of the connection.
    /// The options in `receive_options` will be used when reading the messages that come in on `input_stream`.
    pub fn new(input_stream: T,
               output_stream: U,
               side: ::rpc_twoparty_capnp::Side,
               receive_options: ReaderOptions) -> VatNetwork<T, U>
    {
        let (fulfiller, promise) = oneshot::channel();
        let connection = Connection::new(input_stream, output_stream, side, receive_options, fulfiller);
        let weak_inner = Rc::downgrade(&connection.inner);
        VatNetwork {
            connection: Some(connection),
            weak_connection_inner: weak_inner,
            on_disconnect_promise: promise.fork(),
            side: side,
        }
    }

    /// Returns a promise that resolves when the peer disconnects.
    pub fn on_disconnect(&mut self) -> Promise<(), ::capnp::Error> {
        self.on_disconnect_promise.add_branch()
    }
}

impl <T, U> ::VatNetwork<VatId> for VatNetwork<T, U>
    where T: ::std::io::Read, U: ::std::io::Write
{
    fn connect(&mut self, host_id: VatId) -> Option<Box<::Connection<VatId>>> {
        if host_id == self.side {
            None
        } else {
            let connection = ::std::mem::replace(&mut self.connection, None);
            match connection {
                Some(c) => {
                    Some(Box::new(c))
                } None => {
                    match self.weak_connection_inner.upgrade() {
                        Some(connection_inner) => {
                            Some(Box::new(Connection { inner: connection_inner }))
                        }
                        None => {
                            panic!("tried to reconnect a disconnected twoparty vat network.")
                        }
                    }
                }
            }
        }
    }

    fn accept(&mut self) -> Promise<Box<::Connection<VatId>>, ::capnp::Error> {
        let connection = ::std::mem::replace(&mut self.connection, None);
        match connection {
            Some(c) => Box::new(::futures::future::ok(Box::new(c) as Box<::Connection<VatId>>)),
            None => Box::new(::futures::future::empty()),
        }
    }
}
