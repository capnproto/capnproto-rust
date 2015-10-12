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

pub type VatId = ::rpc_twoparty_capnp::Side;

pub struct IncomingMessage {
    message: ::capnp::message::Reader<::capnp_gj::serialize::OwnedSegments>,
}

impl ::IncomingMessage for IncomingMessage {
    fn get_body<'a>(&'a self) -> ::capnp::Result<::capnp::any_pointer::Reader<'a>> {
        self.message.get_root()
    }
}

pub struct OutgoingMessage {
    message: ::capnp::message::Builder<::capnp::message::HeapAllocator>,
}

impl ::OutgoingMessage for OutgoingMessage {
    fn get_body<'a>(&'a mut self) -> ::capnp::Result<::capnp::any_pointer::Builder<'a>> {
        self.message.get_root()
    }

    fn send(self) {
        unimplemented!()
    }
}

pub struct Connection<T, U> where T: ::gj::io::AsyncRead, U: ::gj::io::AsyncWrite {
    input_stream: T,
    output_stream: U,
    receive_options: ReaderOptions,
}

impl <T, U> Connection<T, U> where T: ::gj::io::AsyncRead, U: ::gj::io::AsyncWrite {
    pub fn new(input_stream: T, output_stream: U, receive_options: ReaderOptions) -> Connection<T, U> {
        Connection { input_stream: input_stream, output_stream: output_stream,
                     receive_options: receive_options }
    }
}

impl <T, U> ::Connection<VatId> for Connection<T, U>
    where T: ::gj::io::AsyncRead, U: ::gj::io::AsyncWrite
{
    fn get_peer_vat_id(&self) -> VatId {
        unimplemented!()
    }

    fn new_outgoing_message(&mut self) -> Box<::OutgoingMessage> {
        unimplemented!()
    }

    fn receive_incoming_message(&mut self) -> ::gj::Promise<Option<Box<::IncomingMessage>>, ::capnp::Error> {
        self.receive_options;
        unimplemented!()
    }

    fn shutdown(&mut self) {
        unimplemented!()
    }
}

// How should this work? I make a two party rpc system by passing in an AsyncStream.
pub struct TwoPartyVatNetwork<T, U> where T: ::gj::io::AsyncRead, U: ::gj::io::AsyncWrite {
    _connection: Option<Connection<T,U>>,
}

impl <T, U> TwoPartyVatNetwork<T, U> where T: ::gj::io::AsyncRead, U: ::gj::io::AsyncWrite {
    pub fn new(input_stream: T, output_stream: U, receive_options: ReaderOptions) -> TwoPartyVatNetwork<T, U> {
        TwoPartyVatNetwork { _connection: Some(Connection::new(input_stream, output_stream, receive_options)) }
    }
}

impl <T, U> ::VatNetwork<VatId> for TwoPartyVatNetwork<T, U>
    where T: ::gj::io::AsyncRead, U: ::gj::io::AsyncWrite
{
    fn connect(&mut self, _host_id: VatId) -> Option<Box<::Connection<VatId>>> {
        unimplemented!()
    }

    fn accept(&mut self) -> ::gj::Promise<Box<::Connection<VatId>>, ::capnp::Error> {
        unimplemented!()
    }
}
