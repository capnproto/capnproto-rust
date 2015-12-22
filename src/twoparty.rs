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

use capnp::message::ReaderOptions;

use std::cell::RefCell;
use std::rc::Rc;

pub type VatId = ::rpc_twoparty_capnp::Side;

pub struct IncomingMessage {
    message: ::capnp::message::Reader<::capnp_gj::serialize::OwnedSegments>,
}

impl IncomingMessage {
    pub fn new(message: ::capnp::message::Reader<::capnp_gj::serialize::OwnedSegments>) -> IncomingMessage {
        IncomingMessage { message: message }
    }
}

impl ::IncomingMessage for IncomingMessage {
    fn get_body<'a>(&'a self) -> ::capnp::Result<::capnp::any_pointer::Reader<'a>> {
        self.message.get_root()
    }
}

pub struct OutgoingMessage<U> where U: ::gj::io::AsyncWrite {
    message: ::capnp::message::Builder<::capnp::message::HeapAllocator>,
    write_queue: Rc<RefCell<::gj::Promise<U, ::capnp::Error>>>,
}

impl <U> ::OutgoingMessage for OutgoingMessage<U> where U: ::gj::io::AsyncWrite {
    fn get_body<'a>(&'a mut self) -> ::capnp::Result<::capnp::any_pointer::Builder<'a>> {
        self.message.get_root()
    }

    fn send(self: Box<Self>) {
        let tmp = *self;
        let OutgoingMessage {message, write_queue} = tmp;
        let queue = ::std::mem::replace(&mut *write_queue.borrow_mut(), ::gj::Promise::never_done());
        *write_queue.borrow_mut() = queue.then(move |s| {
// DEBUG
//            pry!(::capnp::serialize::write_message(&mut ::std::io::stdout(), &message));
            ::capnp_gj::serialize::write_message(s, message).map(move |(s, _)| {
                Ok(s)
            })
        });
    }
}

pub struct Connection<T, U> where T: ::gj::io::AsyncRead, U: ::gj::io::AsyncWrite {
    input_stream: Rc<RefCell<Option<T>>>,
    write_queue: Rc<RefCell<::gj::Promise<U, ::capnp::Error>>>,
    receive_options: ReaderOptions,
}

impl <T, U> Connection<T, U> where T: ::gj::io::AsyncRead, U: ::gj::io::AsyncWrite {
    pub fn new(input_stream: T, output_stream: U, receive_options: ReaderOptions) -> Connection<T, U> {
        Connection { input_stream: Rc::new(RefCell::new(Some(input_stream))),
                     write_queue: Rc::new(RefCell::new(::gj::Promise::ok(output_stream))),
                     receive_options: receive_options }
    }
}

impl <T, U> ::Connection<VatId> for Connection<T, U>
    where T: ::gj::io::AsyncRead, U: ::gj::io::AsyncWrite
{
    fn get_peer_vat_id(&self) -> VatId {
        unimplemented!()
    }

    fn new_outgoing_message(&mut self, _first_segment_word_size: u32) -> Box<::OutgoingMessage> {
        Box::new(OutgoingMessage {
            message: ::capnp::message::Builder::new_default(),
            write_queue: self.write_queue.clone()
        })
    }

    fn receive_incoming_message(&mut self) -> ::gj::Promise<Option<Box<::IncomingMessage>>, ::capnp::Error> {
        self.receive_options;
        let maybe_input_stream = ::std::mem::replace(&mut *self.input_stream.borrow_mut(), None);
        let return_it_here = self.input_stream.clone();
        match maybe_input_stream {
            Some(s) => {
                ::capnp_gj::serialize::try_read_message(s, self.receive_options).map(move |(s, maybe_message)| {
                    *return_it_here.borrow_mut() = Some(s);
                    Ok(maybe_message.map(|message|
                                         Box::new(IncomingMessage::new(message)) as Box<::IncomingMessage>))
                })
            }
            None => panic!(),
        }
    }

    fn shutdown(&mut self) {
        unimplemented!()
    }
}

pub struct VatNetwork<T, U> where T: ::gj::io::AsyncRead, U: ::gj::io::AsyncWrite {
    connection: Option<Connection<T,U>>,
}

impl <T, U> VatNetwork<T, U> where T: ::gj::io::AsyncRead, U: ::gj::io::AsyncWrite {
    pub fn new(input_stream: T, output_stream: U, receive_options: ReaderOptions) -> VatNetwork<T, U> {
        VatNetwork { connection: Some(Connection::new(input_stream, output_stream, receive_options)) }
    }
}

impl <T, U> ::VatNetwork<VatId> for VatNetwork<T, U>
    where T: ::gj::io::AsyncRead, U: ::gj::io::AsyncWrite
{
    fn connect(&mut self, _host_id: VatId) -> Option<Box<::Connection<VatId>>> {
        let connection = ::std::mem::replace(&mut self.connection, None);
        connection.map(|c| Box::new(c) as Box<::Connection<VatId>>)
    }

    fn accept(&mut self) -> ::gj::Promise<Box<::Connection<VatId>>, ::capnp::Error> {
        let connection = ::std::mem::replace(&mut self.connection, None);
        match connection {
            Some(c) => ::gj::Promise::ok(Box::new(c) as Box<::Connection<VatId>>),
            None => unimplemented!(),
        }
    }
}
