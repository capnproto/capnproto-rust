use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

use capnp::capability::{Promise, Response};
use capnp::Error;
use capnp_rpc::{
    auto_reconnect, lazy_auto_reconnect, new_client, new_future_client, rpc_twoparty_capnp,
    twoparty, RpcSystem,
};
use futures::channel::oneshot;
use futures::executor::LocalPool;
use futures::future::Shared;
use futures::task::LocalSpawnExt;
use futures::FutureExt;

use crate::spawn;
use crate::test_capnp::{self, test_interface};

struct TestInterfaceInner {
    error: Option<Error>,
    generation: usize,
    block: Option<Shared<Promise<(), capnp::Error>>>,
}

#[derive(Clone)]
struct TestInterfaceImpl {
    inner: Rc<RefCell<TestInterfaceInner>>,
}

impl TestInterfaceImpl {
    fn new(generation: usize) -> TestInterfaceImpl {
        let inner = TestInterfaceInner {
            generation,
            error: None,
            block: None,
        };
        TestInterfaceImpl {
            inner: Rc::new(RefCell::new(inner)),
        }
    }

    fn set_error(&self, err: capnp::Error) {
        self.inner.borrow_mut().error = Some(err);
    }

    fn block(&self) -> oneshot::Sender<capnp::Result<()>> {
        let (s, r) = oneshot::channel();
        self.inner.borrow_mut().block = Some(
            Promise::from_future(r.map(|ret| match ret {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(err)) => Err(err),
                Err(_) => Err(capnp::Error::failed("dropped sender".into())),
            }))
            .shared(),
        );
        s
    }
}

impl test_interface::Server for TestInterfaceImpl {
    async fn foo(
        self: Rc<Self>,
        params: test_interface::FooParams,
        mut results: test_interface::FooResults,
    ) -> Result<(), Error> {
        if let Some(err) = self.inner.borrow().error.as_ref() {
            return Err(err.clone());
        }
        let params = params.get()?;
        let s = format!(
            "{} {} {}",
            params.get_i(),
            params.get_j(),
            self.inner.borrow().generation
        );
        {
            let mut results = results.get();
            results.set_x(&s[..]);
        }

        let Some(fut) = self.inner.borrow().block.clone() else {
            return Ok(());
        };

        fut.await
    }
}

fn run_until<F>(pool: &mut LocalPool, fut: F) -> Result<String, Error>
where
    F: Future<Output = capnp::Result<Response<test_interface::foo_results::Owned>>>,
{
    match pool.run_until(fut) {
        Ok(resp) => Ok(resp.get()?.get_x()?.to_string()?),
        Err(err) => Err(err),
    }
}

macro_rules! assert_err {
    ($e1:expr, $e2:expr) => {
        let e1 = $e1;
        let e2 = $e2;
        assert_eq!(e1.kind, e2.kind);
        if !e1.extra.ends_with(&e2.extra) {
            assert_eq!(e1.extra, e2.extra);
        }
    };
}

fn test_promise(
    client: &test_interface::Client,
    i: u32,
    j: bool,
) -> Promise<Response<test_interface::foo_results::Owned>, Error> {
    let mut req = client.foo_request();
    req.get().set_i(i);
    req.get().set_j(j);
    req.send().promise
}

fn test(
    pool: &mut LocalPool,
    client: &test_interface::Client,
    i: u32,
    j: bool,
) -> Result<String, Error> {
    let fut = test_promise(client, i, j);
    run_until(pool, fut)
}

fn do_autoconnect_test<F>(pool: &mut LocalPool, wrap_client: F) -> capnp::Result<()>
where
    F: Fn(test_interface::Client) -> test_interface::Client,
{
    let spawner = pool.spawner();

    let (req3, fulfiller, promise1, promise2, promise4) = {
        let connect_count = Rc::new(RefCell::new(0));
        let current_server = Rc::new(RefCell::new(TestInterfaceImpl::new(0)));

        let c_server = current_server.clone();
        let (c, _s) = auto_reconnect(move || {
            let generation = *connect_count.borrow();
            {
                *connect_count.borrow_mut() += 1;
            }
            let server = TestInterfaceImpl::new(generation);
            *c_server.borrow_mut() = server.clone();
            let client: test_interface::Client = new_client(server);
            Ok(client)
        })?;
        let client = wrap_client(c);

        assert_eq!(test(pool, &client, 123, true).unwrap(), "123 true 0");

        current_server
            .borrow()
            .set_error(capnp::Error::disconnected("test1 disconnect".into()));
        assert_err!(
            test(pool, &client, 456, true).unwrap_err(),
            Error::disconnected("test1 disconnect".into())
        );

        assert_eq!(test(pool, &client, 789, false).unwrap(), "789 false 1");
        assert_eq!(test(pool, &client, 21, true).unwrap(), "21 true 1");

        {
            // We cause two disconnect promises to be thrown concurrently. This should only cause the
            // reconnector to reconnect once, not twice.
            let fulfiller = current_server.borrow().block();
            let promise1 = test_promise(&client, 32, false);
            let promise2 = test_promise(&client, 43, true);
            let promise1 = Promise::from_future(spawner.spawn_local_with_handle(promise1).unwrap());
            let promise2 = Promise::from_future(spawner.spawn_local_with_handle(promise2).unwrap());
            pool.run_until_stalled();
            fulfiller
                .send(Err(capnp::Error::disconnected("test2 disconnect".into())))
                .unwrap();
            assert_err!(
                run_until(pool, promise1).expect_err("disconnect error"),
                capnp::Error::disconnected("test2 disconnect".into())
            );
            assert_err!(
                run_until(pool, promise2).expect_err("disconnect error"),
                capnp::Error::disconnected("test2 disconnect".into())
            );
        }

        assert_eq!(test(pool, &client, 43, false).unwrap(), "43 false 2");

        // Start a couple calls that will block at the server end, plus an unsent request.
        let fulfiller = current_server.borrow().block();

        let promise1 = test_promise(&client, 1212, true);
        let promise2 = test_promise(&client, 3434, false);
        let mut req3 = client.foo_request();
        req3.get().set_i(5656);
        req3.get().set_j(true);
        let promise1 = Promise::from_future(spawner.spawn_local_with_handle(promise1).unwrap());
        let promise2 = Promise::from_future(spawner.spawn_local_with_handle(promise2).unwrap());
        pool.run_until_stalled();

        // Now force a reconnect.
        current_server
            .borrow()
            .set_error(capnp::Error::disconnected("test3 disconnect".into()));

        // Initiate a request that will fail with DISCONNECTED.
        let promise4 = test_promise(&client, 7878, false);

        // And throw away our capability entirely, just to make sure that anyone who needs it is holding
        // onto their own ref.
        //client = nullptr;
        (req3, fulfiller, promise1, promise2, promise4)
    };

    // Everything we initiated should still finish.
    assert_err!(
        run_until(pool, promise4).expect_err("disconnect error"),
        capnp::Error::disconnected("test3 disconnect".into())
    );

    // Send the request which we created before the disconnect. There are two behaviors we accept
    // as correct here: it may throw the disconnect exception, or it may automatically redirect to
    // the newly-reconnected destination.
    match run_until(pool, req3.send().promise) {
        Ok(resp) => {
            assert_eq!(resp, "5656 true 3");
        }
        Err(err) => {
            assert_err!(err, capnp::Error::disconnected("test3 disconnect".into()));
        }
    }

    //KJ_EXPECT(!promise1.poll(ws));
    //KJ_EXPECT(!promise2.poll(ws));
    fulfiller.send(Ok(())).unwrap();
    assert_eq!(run_until(pool, promise1).unwrap(), "1212 true 2");
    assert_eq!(run_until(pool, promise2).unwrap(), "3434 false 2");

    Ok(())
}

/// autoReconnect() direct call (exercises newCall() / RequestHook)
#[test]
fn auto_reconnect_direct_call() {
    let mut pool = LocalPool::new();

    do_autoconnect_test(&mut pool, |c| c).unwrap();
}

#[derive(Clone)]
struct Bootstrap(Rc<RefCell<Option<test_interface::Client>>>);

impl Bootstrap {
    fn new() -> Bootstrap {
        Bootstrap(Rc::new(RefCell::new(None)))
    }

    fn set_interface(&self, client: test_interface::Client) {
        *self.0.borrow_mut() = Some(client);
    }
}

impl test_capnp::bootstrap::Server for Bootstrap {
    async fn test_interface(
        self: Rc<Self>,
        _params: test_capnp::bootstrap::TestInterfaceParams,
        mut results: test_capnp::bootstrap::TestInterfaceResults,
    ) -> Result<(), Error> {
        if let Some(client) = self.0.borrow_mut().take() {
            results.get().set_cap(client);
            Ok(())
        } else {
            Err(Error::failed("No interface available".into()))
        }
    }
}

/// autoReconnect() through RPC (exercises call() / CallContextHook)
#[test]
fn auto_reconnect_rpc_call() {
    let (client_writer, server_reader) = async_byte_channel::channel();
    let (server_writer, client_reader) = async_byte_channel::channel();
    let client_network = Box::new(twoparty::VatNetwork::new(
        client_reader,
        client_writer,
        rpc_twoparty_capnp::Side::Client,
        Default::default(),
    ));

    let mut client_rpc_system = RpcSystem::new(client_network, None);

    let server_network = Box::new(twoparty::VatNetwork::new(
        server_reader,
        server_writer,
        rpc_twoparty_capnp::Side::Server,
        Default::default(),
    ));

    let b = Bootstrap::new();
    let bootstrap: test_capnp::bootstrap::Client = capnp_rpc::new_client(b.clone());
    let server_rpc_system = RpcSystem::new(server_network, Some(bootstrap.client));
    let client: test_capnp::bootstrap::Client =
        client_rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
    let disconnector: capnp_rpc::Disconnector<capnp_rpc::rpc_twoparty_capnp::Side> =
        client_rpc_system.get_disconnector();

    let mut pool = LocalPool::new();
    let mut spawner = pool.spawner();
    spawn(&mut spawner, client_rpc_system);
    spawn(&mut spawner, server_rpc_system);

    do_autoconnect_test(&mut pool, |c| {
        b.set_interface(c);
        let req = client.test_interface_request();
        new_future_client(req.send().promise.map(|resp| match resp {
            Ok(resp) => Ok(resp.get()?.get_cap()?),
            Err(err) => Err(err),
        }))
    })
    .unwrap();
    pool.run_until(disconnector).unwrap();
}

/// lazyAutoReconnect() initializes lazily
#[test]
fn lazy_auto_reconnect_test() {
    let mut pool = LocalPool::new();

    let connect_count = Rc::new(RefCell::new(0));
    let current_server = Rc::new(RefCell::new(TestInterfaceImpl::new(0)));

    let c_server = current_server.clone();
    let counter = connect_count.clone();
    let (client, _s) = auto_reconnect(move || {
        let generation = *counter.borrow();
        {
            *counter.borrow_mut() += 1;
        }
        let server = TestInterfaceImpl::new(generation);
        *c_server.borrow_mut() = server.clone();
        let client: test_interface::Client = new_client(server);
        Ok(client)
    })
    .unwrap();

    assert_eq!(*connect_count.borrow(), 1);
    assert_eq!(test(&mut pool, &client, 123, true).unwrap(), "123 true 0");
    assert_eq!(*connect_count.borrow(), 1);

    let c_server = current_server.clone();
    let counter = connect_count.clone();
    let (client, _s) = lazy_auto_reconnect(move || {
        let generation = *counter.borrow();
        {
            *counter.borrow_mut() += 1;
        }
        let server = TestInterfaceImpl::new(generation);
        *c_server.borrow_mut() = server.clone();
        let client: test_interface::Client = new_client(server);
        Ok(client)
    });

    assert_eq!(*connect_count.borrow(), 1);
    assert_eq!(test(&mut pool, &client, 123, true).unwrap(), "123 true 1");
    assert_eq!(*connect_count.borrow(), 2);
    assert_eq!(test(&mut pool, &client, 234, false).unwrap(), "234 false 1");
    assert_eq!(*connect_count.borrow(), 2);

    current_server
        .borrow()
        .set_error(Error::disconnected("test1 disconnect".into()));
    assert_err!(
        test(&mut pool, &client, 345, true).unwrap_err(),
        Error::disconnected("test1 disconnect".into())
    );

    // lazyAutoReconnect is only lazy on the first request, not on reconnects.
    assert_eq!(*connect_count.borrow(), 3);
    assert_eq!(test(&mut pool, &client, 456, false).unwrap(), "456 false 2");
    assert_eq!(*connect_count.borrow(), 3);
}
