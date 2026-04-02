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
//!     async fn identity(
//!         self: Rc<Self>,
//!         params: foo::IdentityParams,
//!         mut results: foo::IdentityResults
//!     ) -> Result<(), ::capnp::Error> {
//!         let x = params.get()?.get_x();
//!         results.get().set_y(x);
//!         Ok(())
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
use std::rc::{Rc, Weak};
use std::task::{Context, Poll};

pub use crate::rpc::Disconnector;
use crate::task_set::TaskSet;

pub use crate::reconnect::{auto_reconnect, lazy_auto_reconnect, SetTarget};

/// Code generated from
/// [rpc.capnp](https://github.com/capnproto/capnproto/blob/master/c%2B%2B/src/capnp/rpc.capnp).
pub mod rpc_capnp;

/// Code generated from
/// [rpc-twoparty.capnp](https://github.com/capnproto/capnproto/blob/master/c%2B%2B/src/capnp/rpc-twoparty.capnp).
pub mod rpc_twoparty_capnp;

/// Like [`try!()`], but for functions that return a [`Promise<T, E>`] rather than a [`Result<T, E>`].
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
mod flow_control;
mod local;
mod queued;
mod reconnect;
mod rpc;
mod sender_queue;
mod split;
mod task_set;
pub mod twoparty;

use capnp::message;

/// A message to be sent by a [`VatNetwork`].
pub trait OutgoingMessage {
    /// Gets the message body, which the caller may fill in any way it wants.
    ///
    /// The standard RPC implementation initializes it as a Message as defined
    /// in `schema/rpc.capnp`.
    fn get_body(&mut self) -> ::capnp::Result<::capnp::any_pointer::Builder<'_>>;

    /// Same as `get_body()`, but returns the corresponding reader type.
    fn get_body_as_reader(&self) -> ::capnp::Result<::capnp::any_pointer::Reader<'_>>;

    /// Sends the message. Returns a promise that resolves once the send has completed.
    /// Dropping the returned promise does *not* cancel the send.
    fn send(
        self: Box<Self>,
    ) -> (
        Promise<(), Error>,
        Rc<message::Builder<message::HeapAllocator>>,
    );

    /// Takes the inner message out of `self`.
    fn take(self: Box<Self>) -> ::capnp::message::Builder<::capnp::message::HeapAllocator>;

    /// Gets the total size of the message, for flow control purposes. Although the caller
    /// could also call get_body().target_size(), doing that would walk the message tree,
    /// whereas typical implementations can compute the size more cheaply by summing
    /// segment sizes.
    fn size_in_words(&self) -> usize;
}

/// A message received from a [`VatNetwork`].
pub trait IncomingMessage {
    /// Gets the message body, to be interpreted by the caller.
    ///
    /// The standard RPC implementation interprets it as a Message as defined
    /// in `schema/rpc.capnp`.
    fn get_body(&self) -> ::capnp::Result<::capnp::any_pointer::Reader<'_>>;
}

/// A two-way RPC connection.
///
/// A connection can be created by [`VatNetwork::connect()`].
pub trait Connection<VatId> {
    /// Returns the connected vat's authenticated VatId.  It is the VatNetwork's
    /// responsibility to authenticate this, so that the caller can be assured
    /// that they are really talking to the identified vat and not an imposter.
    fn get_peer_vat_id(&self) -> VatId;

    /// Allocates a new message to be sent on this connection.
    ///
    /// If `first_segment_word_size` is non-zero, it should be treated as a
    /// hint suggesting how large to make the first segment.  This is entirely
    /// a hint and the connection may adjust it up or down.  If it is zero,
    /// the connection should choose the size itself.
    fn new_outgoing_message(&mut self, first_segment_word_size: u32) -> Box<dyn OutgoingMessage>;

    /// Waits for a message to be received and returns it.  If the read stream cleanly terminates,
    /// returns None. If any other problem occurs, returns an Error.
    fn receive_incoming_message(&mut self) -> Promise<Option<Box<dyn IncomingMessage>>, Error>;

    /// Constructs a flow controller for a new stream on this connection.
    ///
    /// Returns (fc, p), where fc is the new flow controller and p is a promise
    /// that must be polled in order to drive the flow controller.
    fn new_stream(&mut self) -> (Box<dyn FlowController>, Promise<(), Error>) {
        let (fc, f) = crate::flow_control::FixedWindowFlowController::new(
            crate::flow_control::DEFAULT_WINDOW_SIZE,
        );
        (Box::new(fc), f)
    }

    /// Waits until all outgoing messages have been sent, then shuts down the outgoing stream. The
    /// returned promise resolves after shutdown is complete.
    fn shutdown(&mut self, result: ::capnp::Result<()>) -> Promise<(), Error>;
}

/// Tracks a particular RPC stream in order to implement a flow control algorithm.
pub trait FlowController {
    fn send(
        &mut self,
        message: Box<dyn OutgoingMessage>,
        ack: Promise<(), Error>,
    ) -> Promise<(), Error>;
    fn wait_all_acked(&mut self) -> Promise<(), Error>;
}

/// Network facility between vats, it determines how to form connections between
/// vats.
///
/// ## Vat
///
/// Cap'n Proto RPC operates between vats, where a "vat" is some sort of host of
/// objects.  Typically one Cap'n Proto process (in the Unix sense) is one vat.
pub trait VatNetwork<VatId> {
    /// Connects to `host_id`.
    ///
    /// Returns None if `host_id` refers to the local vat.
    fn connect(&mut self, host_id: VatId) -> Option<Box<dyn Connection<VatId>>>;

    /// Waits for the next incoming connection and return it.
    fn accept(&mut self) -> Promise<Box<dyn Connection<VatId>>, ::capnp::Error>;

    /// A promise that cannot be resolved until the shutdown.
    fn drive_until_shutdown(&mut self) -> Promise<(), Error>;
}

/// A portal to objects available on the network.
///
/// The RPC implementation sits on top of an implementation of [`VatNetwork`], which
/// determines how to form connections between vats. The RPC implementation determines
/// how to use such connections to manage object references and make method calls.
///
/// At the moment, this is all rather more general than it needs to be, because the only
/// implementation of `VatNetwork` is [`twoparty::VatNetwork`]. However, eventually we
/// will need to have more sophisticated `VatNetwork` implementations, in order to support
/// [level 3](https://capnproto.org/rpc.html#protocol-features) features.
///
/// An `RpcSystem` is a non-`Send`able `Future` and needs to be driven by a task
/// executor. A common way accomplish that is to pass the `RpcSystem` to
/// `tokio::task::spawn_local()`.
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

    /// Connects to the given vat and returns its bootstrap interface, returns
    /// a client that can be used to invoke the bootstrap interface.
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
    new_client_from_rc(Rc::new(s))
}

/// Variant of `new_client` that works on an `Rc<S>`.
pub fn new_client_from_rc<C, S>(s: Rc<S>) -> C
where
    C: capnp::capability::FromServer<S>,
{
    capnp::capability::FromClientHook::new(Box::new(local::Client::new(
        <C as capnp::capability::FromServer<S>>::from_server(s),
    )))
}

/// Collection of unwrappable capabilities.
///
/// Allows a server to recognize its own capabilities when passed back to it, and obtain the
/// underlying Server objects associated with them. Holds only weak references to Server objects
/// allowing Server objects to be dropped when dropped by the remote client. Call the `gc` method
/// to reclaim memory used for Server objects that have been dropped.
pub struct CapabilityServerSet<S, C>
where
    C: capnp::capability::FromServer<S>,
{
    caps: std::collections::HashMap<usize, Weak<S>>,
    marker: std::marker::PhantomData<C>,
}

impl<S, C> Default for CapabilityServerSet<S, C>
where
    C: capnp::capability::FromServer<S>,
{
    fn default() -> Self {
        Self {
            caps: std::default::Default::default(),
            marker: std::marker::PhantomData,
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
        self.new_client_from_rc(Rc::new(s))
    }

    /// Variant of `new_client` that works on an `Rc<S>`.
    pub fn new_client_from_rc(&mut self, rc: Rc<S>) -> C {
        let weak = Rc::downgrade(&rc);
        let ptr = Rc::as_ptr(&rc) as usize;
        self.caps.insert(ptr, weak);

        let dispatch = <C as capnp::capability::FromServer<S>>::from_server(rc);
        capnp::capability::FromClientHook::new(Box::new(local::Client::new(dispatch)))
    }

    /// Looks up a capability and returns its underlying server object, if found.
    /// Fully resolves the capability before looking it up.
    pub async fn get_local_server(&self, client: &C) -> Option<Rc<S>>
    where
        C: capnp::capability::FromClientHook,
    {
        let resolved: C = capnp::capability::get_resolved_cap(
            capnp::capability::FromClientHook::new(client.as_client_hook().add_ref()),
        )
        .await;
        let hook = resolved.into_client_hook();
        let ptr = hook.get_ptr();
        self.caps.get(&ptr).and_then(|c| c.upgrade())
    }

    /// Looks up a capability and returns its underlying server object, if found.
    /// Does *not* attempt to resolve the capability first, so you will usually want
    /// to call `get_resolved_cap()` before calling this. The advantage of this method
    /// over `get_local_server()` is that this one is synchronous and borrows `self`
    /// over a shorter span (which can be very important if `self` is inside a `RefCell`).
    pub fn get_local_server_of_resolved(&self, client: &C) -> Option<Rc<S>>
    where
        C: capnp::capability::FromClientHook,
    {
        let hook = client.as_client_hook();
        let ptr = hook.get_ptr();
        self.caps.get(&ptr).and_then(|c| c.upgrade())
    }

    /// Reclaim memory used for Server objects that no longer exist.
    pub fn gc(&mut self) {
        self.caps.retain(|_, c| c.strong_count() > 0);
    }
}

/// Creates a `Client` from a future that resolves to a `Client`.
///
/// Any calls that arrive before the resolution are accumulated in a queue.
pub fn new_future_client<T>(
    client_future: impl ::futures::Future<Output = Result<T, Error>> + 'static,
) -> T
where
    T: ::capnp::capability::FromClientHook,
{
    let mut queued_client = crate::queued::Client::new(None);
    let weak_client = Rc::downgrade(&queued_client.inner);

    queued_client.drive(client_future.then(move |r| {
        if let Some(queued_inner) = weak_client.upgrade() {
            crate::queued::ClientInner::resolve(&queued_inner, r.map(|c| c.into_client_hook()));
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
    cap_table: Vec<Option<Box<dyn ::capnp::private::capability::ClientHook>>>,
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

    pub fn set_root<T: ::capnp::traits::Owned>(
        &mut self,
        value: impl ::capnp::traits::SetterInput<T>,
    ) -> ::capnp::Result<()> {
        use capnp::traits::ImbueMut;
        let mut root: ::capnp::any_pointer::Builder = self.builder.get_root()?;
        root.imbue_mut(&mut self.cap_table);
        root.set_as(value)
    }
}

fn canceled_to_error(_e: futures::channel::oneshot::Canceled) -> Error {
    Error::failed("oneshot was canceled".to_string())
}
