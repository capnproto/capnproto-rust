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

//! An implementation of [`VatNetwork`](crate::VatNetwork) for the common case
//! of a client-server connection.

use capnp::capability::Promise;
use capnp::message::ReaderOptions;
use capnp_futures::io::AsyncFdRead;
use futures_channel::oneshot;
use futures_util::TryFutureExt as _;

use std::cell::RefCell;
use std::rc::Rc;

pub type VatId = crate::rpc_twoparty_capnp::Side;

struct IncomingMessage {
    message: ::capnp::message::Reader<capnp::serialize::OwnedSegments>,
}

impl IncomingMessage {
    pub(crate) fn new(message: ::capnp::message::Reader<capnp::serialize::OwnedSegments>) -> Self {
        Self { message }
    }
}

impl crate::IncomingMessage for IncomingMessage {
    fn get_body(&self) -> ::capnp::Result<::capnp::any_pointer::Reader<'_>> {
        self.message.get_root()
    }
}

struct OutgoingMessage {
    message: ::capnp::message::Builder<::capnp::message::HeapAllocator>,
    sender:
        ::capnp_futures::io::Sender<Rc<::capnp::message::Builder<::capnp::message::HeapAllocator>>>,
}

impl crate::OutgoingMessage for OutgoingMessage {
    fn get_body(&mut self) -> ::capnp::Result<::capnp::any_pointer::Builder<'_>> {
        self.message.get_root()
    }

    fn get_body_as_reader(&self) -> ::capnp::Result<::capnp::any_pointer::Reader<'_>> {
        self.message.get_root_as_reader()
    }

    fn send(
        self: Box<Self>,
    ) -> (
        Promise<(), ::capnp::Error>,
        Rc<::capnp::message::Builder<::capnp::message::HeapAllocator>>,
    ) {
        let tmp = *self;
        let Self {
            message,
            mut sender,
        } = tmp;
        let m = Rc::new(message);
        (
            Promise::from_future(sender.send(m.clone()).map_ok(|_| ())),
            m,
        )
    }

    fn take(self: Box<Self>) -> ::capnp::message::Builder<::capnp::message::HeapAllocator> {
        self.message
    }

    fn size_in_words(&self) -> usize {
        self.message.size_in_words()
    }
}

struct ConnectionInner<T>
where
    T: AsyncFdRead + 'static,
{
    input_stream: Rc<RefCell<Option<T>>>,
    sender:
        ::capnp_futures::io::Sender<Rc<::capnp::message::Builder<::capnp::message::HeapAllocator>>>,
    side: crate::rpc_twoparty_capnp::Side,
    receive_options: ReaderOptions,
    on_disconnect_fulfiller: Option<oneshot::Sender<()>>,
    window_size_in_bytes: usize,
}

struct Connection<T>
where
    T: AsyncFdRead + 'static,
{
    inner: Rc<RefCell<ConnectionInner<T>>>,
}

impl<T> Drop for ConnectionInner<T>
where
    T: AsyncFdRead,
{
    fn drop(&mut self) {
        match self.on_disconnect_fulfiller.take() {
            Some(fulfiller) => {
                let _ = fulfiller.send(());
            }
            None => unreachable!(),
        }
    }
}

impl<T> Connection<T>
where
    T: AsyncFdRead,
{
    fn new(
        input_stream: T,
        sender: ::capnp_futures::io::Sender<
            Rc<::capnp::message::Builder<::capnp::message::HeapAllocator>>,
        >,
        side: crate::rpc_twoparty_capnp::Side,
        receive_options: ReaderOptions,
        on_disconnect_fulfiller: oneshot::Sender<()>,
    ) -> Self {
        Self {
            inner: Rc::new(RefCell::new(ConnectionInner {
                input_stream: Rc::new(RefCell::new(Some(input_stream))),
                sender,
                side,
                receive_options,
                on_disconnect_fulfiller: Some(on_disconnect_fulfiller),
                window_size_in_bytes: crate::flow_control::DEFAULT_WINDOW_SIZE,
            })),
        }
    }
}

impl<T> crate::Connection<crate::rpc_twoparty_capnp::Side> for Connection<T>
where
    T: AsyncFdRead,
{
    fn get_peer_vat_id(&self) -> crate::rpc_twoparty_capnp::Side {
        self.inner.borrow().side
    }

    fn new_outgoing_message(
        &mut self,
        first_segment_word_size: u32,
    ) -> Box<dyn crate::OutgoingMessage> {
        let message = ::capnp::message::Builder::new(
            ::capnp::message::HeapAllocator::new().first_segment_words(first_segment_word_size),
        );
        Box::new(OutgoingMessage {
            message,
            sender: self.inner.borrow().sender.clone(),
        })
    }

    fn receive_incoming_message(
        &mut self,
    ) -> Promise<Option<Box<dyn crate::IncomingMessage + 'static>>, ::capnp::Error> {
        let inner = self.inner.borrow_mut();

        let maybe_input_stream = inner.input_stream.borrow_mut().take();
        let return_it_here = inner.input_stream.clone();
        match maybe_input_stream {
            Some(mut s) => {
                let receive_options = inner.receive_options;
                Promise::from_future(async move {
                    let maybe_message =
                        ::capnp_futures::io::serialize::try_read_message(&mut s, receive_options)
                            .await?;
                    *return_it_here.borrow_mut() = Some(s);
                    Ok(maybe_message.map(|message| {
                        Box::new(IncomingMessage::new(message)) as Box<dyn crate::IncomingMessage>
                    }))
                })
            }
            None => {
                Promise::err(::capnp::Error::failed(
                    "this should not be possible".to_string(),
                ))
                //   unreachable!(),
            }
        }
    }

    fn new_stream(&mut self) -> (Box<dyn crate::FlowController>, Promise<(), capnp::Error>) {
        let (fc, f) = crate::flow_control::FixedWindowFlowController::new(
            self.inner.borrow().window_size_in_bytes,
        );
        (Box::new(fc), f)
    }

    fn shutdown(&mut self, result: ::capnp::Result<()>) -> Promise<(), ::capnp::Error> {
        Promise::from_future(self.inner.borrow_mut().sender.terminate(result))
    }
}

pub mod io {
    use std::{
        cell::RefCell,
        future,
        rc::{Rc, Weak},
    };

    use capnp::{capability::Promise, message::ReaderOptions};
    use capnp_futures::io::{AsyncFdRead, AsyncFdWrite};
    use futures_channel::oneshot;
    use futures_util::{FutureExt as _, TryFutureExt as _};

    use crate::twoparty::{Connection, ConnectionInner, VatId};

    /// A two-party vat `VatNetwork` implementation.
    pub struct VatNetwork<T>
    where
        T: AsyncFdRead + 'static,
    {
        // connection handle that we will return on accept()
        connection: Option<Connection<T>>,

        // connection handle that we will return on connect()
        weak_connection_inner: Weak<RefCell<ConnectionInner<T>>>,

        execution_driver: futures_util::future::Shared<Promise<(), ::capnp::Error>>,
        side: crate::rpc_twoparty_capnp::Side,
    }

    /// A two-party vat `VatNetwork` implementation.
    impl<T> VatNetwork<T>
    where
        T: AsyncFdRead,
    {
        /// Creates a new two-party vat network that will receive data on `input_stream` and send data on
        /// `output_stream`. (Typically, performance is best if these streams are buffered, possibly via
        /// `futures::io::BufReader` and `futures::io::BufWriter`.)
        ///
        /// `side` indicates whether this is the client or the server side of the connection. This has no
        /// effect on the data sent over the connection; it merely exists so that `RpcNetwork::bootstrap` knows
        /// whether to return the local or the remote bootstrap capability. `VatId` parameters like this one
        /// will make more sense once we have vat networks with more than two parties.
        ///
        /// The options in `receive_options` will be used when reading the messages that come in on `input_stream`.
        pub fn new<U>(
            input_stream: T,
            output_stream: U,
            side: crate::rpc_twoparty_capnp::Side,
            receive_options: ReaderOptions,
        ) -> Self
        where
            U: AsyncFdWrite + 'static,
        {
            let (fulfiller, disconnect_promise) = oneshot::channel();
            let disconnect_promise =
                disconnect_promise.map_err(|_| ::capnp::Error::disconnected("disconnected".into()));

            let (execution_driver, sender) = {
                let (tx, write_queue) = ::capnp_futures::io::write_queue(output_stream);

                // Don't use `.join()` here because we need to make sure to wait for `disconnect_promise` to
                // resolve even if `write_queue` resolves to an error.
                (
                    Promise::from_future(write_queue.then(move |r| {
                        disconnect_promise
                            .then(move |_| future::ready(r))
                            .map_ok(|_| ())
                    }))
                    .shared(),
                    tx,
                )
            };

            let connection =
                Connection::new(input_stream, sender, side, receive_options, fulfiller);
            let weak_inner = Rc::downgrade(&connection.inner);
            Self {
                connection: Some(connection),
                weak_connection_inner: weak_inner,
                execution_driver,
                side,
            }
        }

        /// Set the number of bytes in the flow control window for each stream created
        /// on this connection.
        pub fn set_window_size(&mut self, window_size: usize) {
            if let Some(ref mut conn) = self.connection {
                conn.inner.borrow_mut().window_size_in_bytes = window_size;
            }
        }
    }

    impl<T> crate::VatNetwork<VatId> for VatNetwork<T>
    where
        T: AsyncFdRead,
    {
        fn connect(&mut self, host_id: VatId) -> Option<Box<dyn crate::Connection<VatId>>> {
            if host_id == self.side {
                None
            } else {
                match self.weak_connection_inner.upgrade() {
                    Some(connection_inner) => Some(Box::new(Connection {
                        inner: connection_inner,
                    })),
                    None => {
                        panic!("tried to reconnect a disconnected twoparty vat network.")
                    }
                }
            }
        }

        fn accept(&mut self) -> Promise<Box<dyn crate::Connection<VatId>>, ::capnp::Error> {
            match self.connection.take() {
                Some(c) => Promise::ok(Box::new(c) as Box<dyn crate::Connection<VatId>>),
                None => Promise::from_future(future::pending()),
            }
        }

        fn drive_until_shutdown(&mut self) -> Promise<(), ::capnp::Error> {
            Promise::from_future(self.execution_driver.clone())
        }
    }
}

#[cfg(feature = "futures-io")]
mod futures_io {
    use capnp::{capability::Promise, message::ReaderOptions};
    use capnp_futures::io::futures_io::Compat;
    use futures_io::{AsyncRead, AsyncWrite};

    use crate::twoparty::VatId;

    /// A two-party vat `VatNetwork` implementation.
    pub struct VatNetwork<T>
    where
        T: AsyncRead + Unpin + 'static,
    {
        inner: crate::twoparty::io::VatNetwork<Compat<T>>,
    }

    impl<T> VatNetwork<T>
    where
        T: AsyncRead + Unpin,
    {
        /// Creates a new two-party vat network that will receive data on `input_stream` and send data on
        /// `output_stream`. (Typically, performance is best if these streams are buffered, possibly via
        /// `futures::io::BufReader` and `futures::io::BufWriter`.)
        ///
        /// `side` indicates whether this is the client or the server side of the connection. This has no
        /// effect on the data sent over the connection; it merely exists so that `RpcNetwork::bootstrap` knows
        /// whether to return the local or the remote bootstrap capability. `VatId` parameters like this one
        /// will make more sense once we have vat networks with more than two parties.
        ///
        /// The options in `receive_options` will be used when reading the messages that come in on `input_stream`.
        pub fn new<U>(
            input_stream: T,
            output_stream: U,
            side: crate::rpc_twoparty_capnp::Side,
            receive_options: ReaderOptions,
        ) -> Self
        where
            U: AsyncWrite + Unpin + 'static,
        {
            VatNetwork {
                inner: crate::twoparty::io::VatNetwork::new(
                    Compat::new(input_stream),
                    Compat::new(output_stream),
                    side,
                    receive_options,
                ),
            }
        }

        /// Set the number of bytes in the flow control window for each stream created
        /// on this connection.
        pub fn set_window_size(&mut self, window_size: usize) {
            self.inner.set_window_size(window_size)
        }
    }

    impl<T> crate::VatNetwork<VatId> for VatNetwork<T>
    where
        T: AsyncRead + Unpin,
    {
        fn connect(&mut self, host_id: VatId) -> Option<Box<dyn crate::Connection<VatId>>> {
            self.inner.connect(host_id)
        }

        fn accept(&mut self) -> Promise<Box<dyn crate::Connection<VatId>>, capnp::Error> {
            self.inner.accept()
        }

        fn drive_until_shutdown(&mut self) -> Promise<(), capnp::Error> {
            self.inner.drive_until_shutdown()
        }
    }
}

#[cfg(feature = "futures-io")]
pub use futures_io::VatNetwork;
