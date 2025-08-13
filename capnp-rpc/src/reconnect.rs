use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use capnp::capability::{FromClientHook, Promise};
use capnp::private::capability::{ClientHook, RequestHook};
use futures::TryFutureExt;

/// Trait implemented by the reconnecting client to set new connection out-of-band.
///
/// When using [`auto_reconnect`] or [`lazy_auto_reconnect`] it is not always optimal
/// to wait for a call to fail with [`Disconnected`](capnp::ErrorKind::Disconnected)
/// before replacing the client that is wrapped with a new fresh one.
///
/// Sometimes we know by other means that a client has gone away. It could be that we
/// have clients that automatically sends us a new capability when it reconnects to us.
///
/// For these situations you can use the implementation of this trait that you get from
/// [`auto_reconnect`] or [`lazy_auto_reconnect`] to manually set the target of the
/// wrapped client.
///
/// # Example
///
/// ```ignore
/// // The reconnecting client that automatically calls connect
/// let (foo_client, set_target) = auto_reconnect(|| {
///     Ok(new_future_client(connect()))
/// })?;
///
/// // do work with foo_client
/// ...
///
/// // We become aware that the client has gone so reconnect manually
/// set_target.set_target(new_future_client(connect()));
///
/// // do more work with foo_client
/// ...
/// ```
pub trait SetTarget<C> {
    /// Adds a new reference to this implementation of SetTarget.
    ///
    /// This is mostly to get around that `Clone` requires `Sized` and so you need this
    /// trick to get a copy of the `Box<dyn SetTarget<C>>` you got from making the
    /// reconnecting client.
    fn add_ref(&self) -> Box<dyn SetTarget<C>>;

    /// Sets the target client of the reconnecting client that this trait implementation is
    /// for.
    fn set_target(&self, target: C);
}

impl<C> Clone for Box<dyn SetTarget<C>> {
    fn clone(&self) -> Self {
        self.add_ref()
    }
}

struct ClientInner<F, C> {
    connect: F,
    current: Option<Box<dyn ClientHook>>,
    generation: usize,
    marker: PhantomData<C>,
}

impl<F, C> ClientInner<F, C>
where
    F: FnMut() -> capnp::Result<C>,
    F: 'static,
    C: FromClientHook,
{
    fn get_current(&mut self) -> Box<dyn ClientHook> {
        if let Some(hook) = self.current.as_ref() {
            hook.add_ref()
        } else {
            let hook = match (self.connect)() {
                Ok(hook) => hook.into_client_hook(),
                Err(err) => crate::broken::new_cap(err),
            };
            self.current = Some(hook.add_ref());
            hook
        }
    }
}

struct Client<F, C> {
    inner: Rc<RefCell<ClientInner<F, C>>>,
}

impl<F, C> Client<F, C>
where
    F: FnMut() -> capnp::Result<C>,
    F: 'static,
    C: FromClientHook,
    C: 'static,
{
    pub fn new(connect: F) -> Client<F, C> {
        Client {
            inner: Rc::new(RefCell::new(ClientInner {
                connect,
                generation: 0,
                current: None,
                marker: PhantomData,
            })),
        }
    }

    pub fn get_current(&self) -> Box<dyn ClientHook> {
        self.inner.borrow_mut().get_current()
    }

    fn wrap<T: 'static>(&self, promise: Promise<T, capnp::Error>) -> Promise<T, capnp::Error> {
        let c = self.clone();
        let generation = self.inner.borrow().generation;
        Promise::from_future(promise.map_err(move |err| {
            if err.kind == capnp::ErrorKind::Disconnected
                && generation == c.inner.borrow().generation
            {
                let mut inner = c.inner.borrow_mut();
                inner.generation = generation + 1;
                match (inner.connect)() {
                    Ok(hook) => inner.current = Some(hook.into_client_hook()),
                    Err(err) => inner.current = Some(crate::broken::new_cap(err)),
                }
            }
            err
        }))
    }
}

impl<F: 'static, C> SetTarget<C> for Client<F, C>
where
    F: 'static,
    C: FromClientHook,
    C: 'static,
{
    fn add_ref(&self) -> Box<dyn SetTarget<C>> {
        Box::new(self.clone())
    }

    fn set_target(&self, target: C) {
        self.inner.borrow_mut().current = Some(target.into_client_hook());
    }
}

impl<F, C> Clone for Client<F, C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<F, C> ClientHook for Client<F, C>
where
    F: FnMut() -> capnp::Result<C>,
    F: 'static,
    C: FromClientHook,
    C: 'static,
{
    fn add_ref(&self) -> Box<dyn ClientHook> {
        Box::new(self.clone())
    }

    fn new_call(
        &self,
        interface_id: u64,
        method_id: u16,
        size_hint: Option<capnp::MessageSize>,
    ) -> capnp::capability::Request<capnp::any_pointer::Owned, capnp::any_pointer::Owned> {
        let result = self
            .get_current()
            .new_call(interface_id, method_id, size_hint);
        let hook = Request::new(self.clone(), result.hook);
        capnp::capability::Request::new(Box::new(hook))
    }

    fn call(
        &self,
        interface_id: u64,
        method_id: u16,
        params: Box<dyn capnp::private::capability::ParamsHook>,
        results: Box<dyn capnp::private::capability::ResultsHook>,
    ) -> Promise<(), capnp::Error> {
        let result = self
            .get_current()
            .call(interface_id, method_id, params, results);
        self.wrap(result)
    }

    fn get_brand(&self) -> usize {
        0
    }

    fn get_ptr(&self) -> usize {
        (self.inner.as_ref()) as *const _ as usize
    }

    fn get_resolved(&self) -> Option<Box<dyn ClientHook>> {
        None
    }

    fn when_more_resolved(&self) -> Option<Promise<Box<dyn ClientHook>, capnp::Error>> {
        None
    }

    fn when_resolved(&self) -> Promise<(), capnp::Error> {
        Promise::ok(())
    }
}

struct Request<F, C> {
    parent: Client<F, C>,
    inner: Box<dyn RequestHook>,
}

impl<F, C> Request<F, C> {
    fn new(parent: Client<F, C>, inner: Box<dyn RequestHook>) -> Request<F, C> {
        Request { parent, inner }
    }
}

impl<F, C> RequestHook for Request<F, C>
where
    F: FnMut() -> capnp::Result<C>,
    F: 'static,
    C: FromClientHook,
    C: 'static,
{
    fn get(&mut self) -> capnp::any_pointer::Builder<'_> {
        self.inner.get()
    }

    fn get_brand(&self) -> usize {
        0
    }

    fn send(self: Box<Self>) -> capnp::capability::RemotePromise<capnp::any_pointer::Owned> {
        let parent = self.parent;
        let mut result = self.inner.send();
        result.promise = parent.wrap(result.promise);
        result
    }

    fn send_streaming(self: Box<Self>) -> Promise<(), capnp::Error> {
        todo!()
    }

    fn tail_send(
        self: Box<Self>,
    ) -> Option<(
        u32,
        Promise<(), capnp::Error>,
        Box<dyn capnp::private::capability::PipelineHook>,
    )> {
        todo!()
    }
}

/// Creates a new client that reconnects when getting [`ErrorKind::Disconnected`](capnp::ErrorKind::Disconnected) errors.
///
/// Usually when you get a [`Disconnected`](capnp::ErrorKind::Disconnected) error response from calling a method on a capability
/// it means the end of that capability for good. And so you can't call methods on that
/// capability any more.
///
/// When you have a way of getting the capability back: Be it from a bootstrap or because
/// the capability is persistent this method can help you wrap that reconnection logic into a client
/// that automatically runs the logic whenever a method call returns [`Disconnected`](capnp::ErrorKind::Disconnected).
///
/// The way it works is that you provide a closure that returns a fresh client or a permanent error and
/// you get a new connected client and a [`SetTarget`] interface that you can optionally use to prematurely
/// replace the client.
///
/// There is one caveat though: The original request that got a [`Disconnected`](capnp::ErrorKind::Disconnected)
/// will still get that response. It is up to the caller to retry the call if relevant. `auto_reconnect`` only
/// deals with the calls that come after.
///
/// # Example
///
/// ```capnp
/// # Cap'n Proto schema
/// interface Foo {
///     identity @0 (x: UInt32) -> (y: UInt32);
/// }
/// ```
///
/// ```ignore
/// // A simple bootstrapped tcp connection to remote.example.com
/// async fn connect() -> capnp::Result<foo_client::Client> {
///     let stream = tokio::net::TcpStream::connect(&"remote.example.com:3001").await?;
///     stream.set_nodelay(true)?;
///     let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
///
///     let network = Box::new(twoparty::VatNetwork::new(
///         futures::io::BufReader::new(reader),
///         futures::io::BufWriter::new(writer),
///         rpc_twoparty_capnp::Side::Client,
///         Default::default(),
///     ));
///
///     let mut rpc_system = RpcSystem::new(network, None);
///     let foo_client: foo_client::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
///     tokio::task::spawn_local(rpc_system);
///     Ok(foo_client)
/// }
/// // The reconnecting client that automatically calls connect
/// let (foo_client, _) = auto_reconnect(|| {
///     // By using new_future_client we delay any calls until we have a new connection.
///     Ok(new_future_client(connect()))
/// })?;
/// // Calling Foo like normally.
/// let mut request = foo_client.identity_request();
/// request.get().set_x(123);
/// let promise = request.send().promise.and_then(|response| {
///     println!("results = {}", response.get()?.get_y());
///     Ok(())
/// });
/// ```
pub fn auto_reconnect<F, C>(mut connect: F) -> capnp::Result<(C, Box<dyn SetTarget<C>>)>
where
    F: FnMut() -> capnp::Result<C>,
    F: 'static,
    C: FromClientHook,
    C: 'static,
{
    let current = connect()?;
    let c = Client::new(connect);
    c.set_target(current);
    let hook: Box<dyn ClientHook> = Box::new(c.clone());
    Ok((FromClientHook::new(hook), Box::new(c)))
}

/// Creates a new client that lazily connect and also reconnects when getting [`ErrorKind::Disconnected`](capnp::ErrorKind::Disconnected) errors.
///
/// For explanation of how this functions see: [`auto_reconnect`]
///
/// The main difference between [`auto_reconnect`] and this function is that while [`auto_reconnect`] will call
/// the closure immediately to get an inner client to wrap, this function starts out disconnected and only calls
/// the closure to get the actual client when the capability is first used.
pub fn lazy_auto_reconnect<F, C>(connect: F) -> (C, Box<dyn SetTarget<C>>)
where
    F: FnMut() -> capnp::Result<C>,
    F: 'static,
    C: FromClientHook,
    C: 'static,
{
    let c: Client<F, C> = Client::new(connect);
    let hook: Box<dyn ClientHook> = Box::new(c.clone());
    (FromClientHook::new(hook), Box::new(c))
}
