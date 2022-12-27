// Copyright (c) 2013-2017 Sandstorm Development Group, Inc. and contributors
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

//! An implementation of the [Cap'n Proto remote procedure call](https://capnproto.org/rpc.html)
//! protocol. Includes all [Level 1](https://capnproto.org/rpc.html#protocol-features) features.
//!
//! # Example
//!
//! ```capnp
//! # Cap'n Proto schema
//! interface Foo {
//!     identity @0 (x: UInt32) -> (y: UInt32);
//! }
//! ```
//!
//! ```ignore
//! // Rust server defining an implementation of Foo.
//! struct FooImpl;
//! impl foo::Server for FooImpl {
//!     fn identity(&mut self,
//!                 params: foo::IdentityParams,
//!                 mut results: foo::IdentityResults)
//!                 -> Promise<(), ::capnp::Error>
//!     {
//!         let x = pry!(params.get()).get_x();
//!         results.get().set_y(x);
//!         Promise::ok(())
//!     }
//! }
//! ```
//!
//! ```ignore
//! // Rust client calling a remote implementation of Foo.
//! let mut request = foo_client.identity_request();
//! request.get().set_x(123);
//! let promise = request.send().promise.and_then(|response| {
//!     println!("results = {}", response.get()?.get_y());
//!     Ok(())
//! });
//! ```
//!
//! For a more complete example, see <https://github.com/capnproto/capnproto-rust/tree/master/capnp-rpc/examples/calculator>

use capnp::capability::Promise;
use capnp::private::capability::ClientHook;
use capnp::Error;
use futures::channel::oneshot;
use futures::{Future, FutureExt, TryFutureExt};
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

pub use crate::rpc::Disconnector;
use crate::task_set::TaskSet;

/// Code generated from
/// [rpc.capnp](https://github.com/sandstorm-io/capnproto/blob/master/c%2B%2B/src/capnp/rpc.capnp).
pub mod rpc_capnp;

/// Code generated from
/// [rpc-twoparty.capnp](https://github.com/sandstorm-io/capnproto/blob/master/c%2B%2B/src/capnp/rpc-twoparty.capnp).
pub mod rpc_twoparty_capnp;

/// Like `try!()`, but for functions that return a `Promise<T, E>` rather than a `Result<T, E>`.
///
/// Unwraps a `Result<T, E>`. In the case of an error `Err(e)`, immediately returns from the
/// enclosing function with `Promise::err(e)`.
#[macro_export]
macro_rules! pry {
    ($expr:expr) => {
        match $expr {
            ::std::result::Result::Ok(val) => val,
            ::std::result::Result::Err(err) => {
                return ::capnp::capability::Promise::err(::std::convert::From::from(err))
            }
        }
    };
}

mod attach;
mod broken;
mod local;
mod queued;
mod rpc;
mod sender_queue;
mod split;
mod task_set;
pub mod twoparty;

pub trait OutgoingMessage {
    fn get_body(&mut self) -> ::capnp::Result<::capnp::any_pointer::Builder>;
    fn get_body_as_reader(&self) -> ::capnp::Result<::capnp::any_pointer::Reader>;

    /// Sends the message. Returns a promise for the message that resolves once the send has completed.
    /// Dropping the returned promise does *not* cancel the send.
    fn send(
        self: Box<Self>,
    ) -> (
        Promise<Rc<::capnp::message::Builder<::capnp::message::HeapAllocator>>, ::capnp::Error>,
        Rc<::capnp::message::Builder<::capnp::message::HeapAllocator>>,
    );

    fn take(self: Box<Self>) -> ::capnp::message::Builder<::capnp::message::HeapAllocator>;
}

pub trait IncomingMessage {
    fn get_body(&self) -> ::capnp::Result<::capnp::any_pointer::Reader>;
}

pub trait Connection<VatId> {
    fn get_peer_vat_id(&self) -> VatId;
    fn new_outgoing_message(&mut self, first_segment_word_size: u32) -> Box<dyn OutgoingMessage>;

    /// Waits for a message to be received and returns it.  If the read stream cleanly terminates,
    /// returns None. If any other problem occurs, returns an Error.
    fn receive_incoming_message(&mut self) -> Promise<Option<Box<dyn IncomingMessage>>, Error>;

    // Waits until all outgoing messages have been sent, then shuts down the outgoing stream. The
    // returned promise resolves after shutdown is complete.
    fn shutdown(&mut self, result: ::capnp::Result<()>) -> Promise<(), Error>;
}

pub trait VatNetwork<VatId> {
    /// Returns None if `hostId` refers to the local vat.
    fn connect(&mut self, host_id: VatId) -> Option<Box<dyn Connection<VatId>>>;

    /// Waits for the next incoming connection and return it.
    fn accept(&mut self) -> Promise<Box<dyn Connection<VatId>>, ::capnp::Error>;

    fn drive_until_shutdown(&mut self) -> Promise<(), Error>;
}

/// A portal to objects available on the network.
///
/// The RPC implemententation sits on top of an implementation of `VatNetwork`, which
/// determines how to form connections between vats. The RPC implementation determines
/// how to use such connections to manage object references and make method calls.
///
/// At the moment, this is all rather more general than it needs to be, because the only
/// implementation of `VatNetwork` is `twoparty::VatNetwork`. However, eventually we
/// will need to have more sophisticated `VatNetwork` implementations, in order to support
/// [level 3](https://capnproto.org/rpc.html#protocol-features) features.
///
/// An `RpcSystem` is a `Future` and needs to be driven by a task executor. A common way
/// accomplish that is to pass the `RpcSystem` to `tokio_core::reactor::Handle::spawn()`.
#[must_use = "futures do nothing unless polled"]
pub struct RpcSystem<VatId>
where
    VatId: 'static,
{
    network: Box<dyn crate::VatNetwork<VatId>>,

    bootstrap_cap: Box<dyn ClientHook>,

    // XXX To handle three or more party networks, this should be a map from connection pointers
    // to connection states.
    connection_state: Rc<RefCell<Option<Rc<rpc::ConnectionState<VatId>>>>>,

    tasks: TaskSet<Error>,
    handle: crate::task_set::TaskSetHandle<Error>,
}

impl<VatId> RpcSystem<VatId> {
    /// Constructs a new `RpcSystem` with the given network and bootstrap capability.
    pub fn new(
        mut network: Box<dyn crate::VatNetwork<VatId>>,
        bootstrap: Option<::capnp::capability::Client>,
    ) -> Self {
        let bootstrap_cap = match bootstrap {
            Some(cap) => cap.hook,
            None => broken::new_cap(Error::failed("no bootstrap capability".to_string())),
        };
        let (mut handle, tasks) = TaskSet::new(Box::new(SystemTaskReaper));

        let mut handle1 = handle.clone();
        handle.add(network.drive_until_shutdown().then(move |r| {
            let r = match r {
                Ok(()) => Ok(()),
                Err(e) => {
                    if e.kind != ::capnp::ErrorKind::Disconnected {
                        // Don't report disconnects as an error.
                        Err(e)
                    } else {
                        Ok(())
                    }
                }
            };

            handle1.terminate(r);
            Promise::ok(())
        }));

        let mut result = Self {
            network,
            bootstrap_cap,
            connection_state: Rc::new(RefCell::new(None)),

            tasks,
            handle: handle.clone(),
        };

        let accept_loop = result.accept_loop();
        handle.add(accept_loop);
        result
    }

    /// Connects to the given vat and returns its bootstrap interface.
    pub fn bootstrap<T>(&mut self, vat_id: VatId) -> T
    where
        T: ::capnp::capability::FromClientHook,
    {
        let Some(connection) = self.network.connect(vat_id) else {
            return T::new(self.bootstrap_cap.clone());
        };
        let connection_state = Self::get_connection_state(
            &self.connection_state,
            self.bootstrap_cap.clone(),
            connection,
            self.handle.clone(),
        );

        let hook = rpc::ConnectionState::bootstrap(&connection_state);
        T::new(hook)
    }

    // not really a loop, because it doesn't need to be for the two party case
    fn accept_loop(&mut self) -> Promise<(), Error> {
        let connection_state_ref = self.connection_state.clone();
        let bootstrap_cap = self.bootstrap_cap.clone();
        let handle = self.handle.clone();
        Promise::from_future(self.network.accept().map_ok(move |connection| {
            Self::get_connection_state(&connection_state_ref, bootstrap_cap, connection, handle);
        }))
    }

    // If `connection_state_ref` is not already populated, populates it with a new
    // `ConnectionState` built from a local bootstrap capability and `connection`,
    // spawning any background tasks onto `handle`. Returns the resulting value
    // held in `connection_state_ref`.
    fn get_connection_state(
        connection_state_ref: &Rc<RefCell<Option<Rc<rpc::ConnectionState<VatId>>>>>,
        bootstrap_cap: Box<dyn ClientHook>,
        connection: Box<dyn crate::Connection<VatId>>,
        mut handle: crate::task_set::TaskSetHandle<Error>,
    ) -> Rc<rpc::ConnectionState<VatId>> {
        // TODO this needs to be updated once we allow more general VatNetworks.
        let (tasks, result) = match *connection_state_ref.borrow() {
            Some(ref connection_state) => {
                // return early.
                return connection_state.clone();
            }
            None => {
                let (on_disconnect_fulfiller, on_disconnect_promise) =
                    oneshot::channel::<Promise<(), Error>>();
                let connection_state_ref1 = connection_state_ref.clone();
                handle.add(on_disconnect_promise.then(move |shutdown_promise| {
                    *connection_state_ref1.borrow_mut() = None;
                    match shutdown_promise {
                        Ok(s) => s,
                        Err(e) => Promise::err(Error::failed(format!("{e}"))),
                    }
                }));
                rpc::ConnectionState::new(bootstrap_cap, connection, on_disconnect_fulfiller)
            }
        };
        *connection_state_ref.borrow_mut() = Some(result.clone());
        handle.add(tasks);
        result
    }

    /// Returns a `Disconnector` future that can be run to cleanly close the connection to this `RpcSystem`'s network.
    /// You should get the `Disconnector` before you spawn the `RpcSystem`.
    pub fn get_disconnector(&self) -> rpc::Disconnector<VatId> {
        rpc::Disconnector::new(self.connection_state.clone())
    }
}

impl<VatId> Future for RpcSystem<VatId>
where
    VatId: 'static,
{
    type Output = Result<(), Error>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.tasks).poll(cx)
    }
}

/// Creates a new local RPC client of type `C` out of an object that implements a server trait `S`.
pub fn new_client<C, S>(s: S) -> C
where
    C: capnp::capability::FromServer<S>,
{
    capnp::capability::FromClientHook::new(Box::new(local::Client::new(
        <C as capnp::capability::FromServer<S>>::from_server(s),
    )))
}

/// Allows a server to recognize its own capabilities when passed back to it, and obtain the
/// underlying Server objects associated with them.
pub struct CapabilityServerSet<S, C>
where
    C: capnp::capability::FromServer<S>,
{
    caps: std::collections::HashMap<usize, Rc<RefCell<C::Dispatch>>>,
}

impl<S, C> Default for CapabilityServerSet<S, C>
where
    C: capnp::capability::FromServer<S>,
{
    fn default() -> Self {
        Self {
            caps: std::default::Default::default(),
        }
    }
}

impl<S, C> CapabilityServerSet<S, C>
where
    C: capnp::capability::FromServer<S>,
{
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a new capability to the set and returns a client backed by it.
    pub fn new_client(&mut self, s: S) -> C {
        let dispatch = <C as capnp::capability::FromServer<S>>::from_server(s);
        let wrapped = Rc::new(RefCell::new(dispatch));
        let ptr = wrapped.as_ptr() as usize;
        self.caps.insert(ptr, wrapped.clone());
        capnp::capability::FromClientHook::new(Box::new(local::Client::from_rc(wrapped)))
    }

    /// Looks up a capability and returns its underlying server object, if found.
    /// Fully resolves the capability before looking it up.
    pub async fn get_local_server(&self, client: &C) -> Option<&Rc<RefCell<C::Dispatch>>>
    where
        C: capnp::capability::FromClientHook,
    {
        let resolved: C = capnp::capability::get_resolved_cap(
            capnp::capability::FromClientHook::new(client.as_client_hook().add_ref()),
        )
        .await;
        let hook = resolved.into_client_hook();
        let ptr = hook.get_ptr();
        self.caps.get(&ptr)
    }

    /// Looks up a capability and returns its underlying server object, if found.
    /// Does *not* attempt to resolve the capability first, so you will usually want
    /// to call `get_resolved_cap()` before calling this. The advantage of this method
    /// over `get_local_server()` is that this one is synchronous and borrows `self`
    /// over a shorter span (which can be very important if `self` is inside a `RefCell`).
    pub fn get_local_server_of_resolved(&self, client: &C) -> Option<&Rc<RefCell<C::Dispatch>>>
    where
        C: capnp::capability::FromClientHook,
    {
        let hook = client.as_client_hook();
        let ptr = hook.get_ptr();
        self.caps.get(&ptr)
    }
}

/// Converts a promise for a client into a client that queues up any calls that arrive
/// before the promise resolves.
// TODO: figure out a better way to allow construction of promise clients.
pub fn new_promise_client<T, F>(client_promise: F) -> T
where
    T: ::capnp::capability::FromClientHook,
    F: ::futures::Future<Output = Result<capnp::capability::Client, Error>>,
    F: 'static + Unpin,
{
    let mut queued_client = crate::queued::Client::new(None);
    let weak_client = Rc::downgrade(&queued_client.inner);

    queued_client.drive(client_promise.then(move |r| {
        if let Some(queued_inner) = weak_client.upgrade() {
            crate::queued::ClientInner::resolve(&queued_inner, r.map(|c| c.hook));
        }
        Promise::ok(())
    }));

    T::new(Box::new(queued_client))
}

struct SystemTaskReaper;
impl crate::task_set::TaskReaper<Error> for SystemTaskReaper {
    fn task_failed(&mut self, error: Error) {
        println!("ERROR: {error}");
    }
}

pub struct ImbuedMessageBuilder<A>
where
    A: ::capnp::message::Allocator,
{
    builder: ::capnp::message::Builder<A>,
    cap_table: Vec<Option<Box<dyn (::capnp::private::capability::ClientHook)>>>,
}

impl<A> ImbuedMessageBuilder<A>
where
    A: ::capnp::message::Allocator,
{
    pub fn new(allocator: A) -> Self {
        Self {
            builder: ::capnp::message::Builder::new(allocator),
            cap_table: Vec::new(),
        }
    }

    pub fn get_root<'a, T>(&'a mut self) -> ::capnp::Result<T>
    where
        T: ::capnp::traits::FromPointerBuilder<'a>,
    {
        use capnp::traits::ImbueMut;
        let mut root: ::capnp::any_pointer::Builder = self.builder.get_root()?;
        root.imbue_mut(&mut self.cap_table);
        root.get_as()
    }

    pub fn set_root<From>(&mut self, value: From) -> ::capnp::Result<()>
    where
        From: ::capnp::traits::SetPointerBuilder,
    {
        use capnp::traits::ImbueMut;
        let mut root: ::capnp::any_pointer::Builder = self.builder.get_root()?;
        root.imbue_mut(&mut self.cap_table);
        root.set_as(value)
    }
}

fn canceled_to_error(_e: futures::channel::oneshot::Canceled) -> Error {
    Error::failed("oneshot was canceled".to_string())
}
