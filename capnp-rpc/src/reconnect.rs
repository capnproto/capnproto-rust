use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use capnp::capability::{FromClientHook, Promise};
use capnp::private::capability::{ClientHook, RequestHook};
use futures::TryFutureExt;

pub trait SetTarget<C> {
    fn add_ref(&self) -> Box<dyn SetTarget<C>>;
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
