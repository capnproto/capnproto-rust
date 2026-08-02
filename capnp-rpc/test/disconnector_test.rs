//! Tests that a `Disconnector` future resolves only once the connection's
//! `shutdown()` has completed, and that it propagates shutdown errors.
//! See https://github.com/capnproto/capnproto-rust/issues/583

use std::cell::Cell;
use std::rc::Rc;

use capnp::capability::Promise;
use capnp::Error;
use capnp_rpc::rpc_twoparty_capnp::Side;
use capnp_rpc::{Connection, IncomingMessage, OutgoingMessage, RpcSystem, VatNetwork};

use futures::channel::oneshot;
use futures::executor::LocalPool;
use futures::task::LocalSpawnExt;
use futures::FutureExt;

struct MockOutgoingMessage {
    message: ::capnp::message::Builder<::capnp::message::HeapAllocator>,
}

impl OutgoingMessage for MockOutgoingMessage {
    fn get_body(&mut self) -> ::capnp::Result<::capnp::any_pointer::Builder<'_>> {
        self.message.get_root()
    }

    fn get_body_as_reader(&self) -> ::capnp::Result<::capnp::any_pointer::Reader<'_>> {
        self.message.get_root_as_reader()
    }

    fn send(
        self: Box<Self>,
    ) -> (
        Promise<(), Error>,
        Rc<::capnp::message::Builder<::capnp::message::HeapAllocator>>,
    ) {
        (Promise::ok(()), Rc::new(self.message))
    }

    fn take(self: Box<Self>) -> ::capnp::message::Builder<::capnp::message::HeapAllocator> {
        self.message
    }

    fn size_in_words(&self) -> usize {
        self.message.size_in_words()
    }
}

/// A connection that never receives any messages and whose `shutdown()`
/// completes with whatever result is sent on the `shutdown_result` channel.
struct MockConnection {
    shutdown_result: Option<oneshot::Receiver<Result<(), Error>>>,
    shutdown_called: Rc<Cell<bool>>,
}

impl Connection<Side> for MockConnection {
    fn get_peer_vat_id(&self) -> Side {
        Side::Server
    }

    fn new_outgoing_message(&mut self, _first_segment_word_size: u32) -> Box<dyn OutgoingMessage> {
        Box::new(MockOutgoingMessage {
            message: ::capnp::message::Builder::new_default(),
        })
    }

    fn receive_incoming_message(&mut self) -> Promise<Option<Box<dyn IncomingMessage>>, Error> {
        Promise::from_future(futures::future::pending())
    }

    fn shutdown(&mut self, _result: ::capnp::Result<()>) -> Promise<(), Error> {
        self.shutdown_called.set(true);
        match self.shutdown_result.take() {
            Some(rx) => Promise::from_future(async move {
                match rx.await {
                    Ok(result) => result,
                    Err(_) => Err(Error::failed("shutdown result sender was dropped".into())),
                }
            }),
            None => Promise::err(Error::failed("shutdown() called twice".into())),
        }
    }
}

struct MockNetwork {
    connection: Option<Box<dyn Connection<Side>>>,
}

impl VatNetwork<Side> for MockNetwork {
    fn connect(&mut self, _host_id: Side) -> Option<Box<dyn Connection<Side>>> {
        self.connection.take()
    }

    fn accept(&mut self) -> Promise<Box<dyn Connection<Side>>, Error> {
        Promise::from_future(futures::future::pending())
    }

    fn drive_until_shutdown(&mut self) -> Promise<(), Error> {
        Promise::from_future(futures::future::pending())
    }
}

fn mock_setup() -> (
    LocalPool,
    oneshot::Sender<Result<(), Error>>,
    Rc<Cell<bool>>,
    futures::future::RemoteHandle<Result<(), Error>>,
) {
    let mut pool = LocalPool::new();
    let spawner = pool.spawner();

    let (tx, rx) = oneshot::channel();
    let shutdown_called = Rc::new(Cell::new(false));
    let network = Box::new(MockNetwork {
        connection: Some(Box::new(MockConnection {
            shutdown_result: Some(rx),
            shutdown_called: shutdown_called.clone(),
        })),
    });

    let mut rpc_system = RpcSystem::new(network, None);
    let disconnector = rpc_system.get_disconnector();

    // Trigger creation of the connection state.
    let _client: crate::test_capnp::bootstrap::Client = rpc_system.bootstrap(Side::Server);

    // The RpcSystem reports the shutdown error too; ignore it so that the
    // spawned task does not panic in the error test.
    spawner.spawn_local(rpc_system.map(|_| ())).unwrap();

    let disconnector_handle = spawner.spawn_local_with_handle(disconnector).unwrap();

    // Run until all tasks are blocked. The disconnector must not resolve,
    // because the mock shutdown has not completed yet.
    pool.run_until_stalled();
    assert!(shutdown_called.get());

    (pool, tx, shutdown_called, disconnector_handle)
}

#[test]
fn disconnector_waits_for_connection_shutdown() {
    let (mut pool, tx, _shutdown_called, mut disconnector_handle) = mock_setup();

    assert!(
        (&mut disconnector_handle).now_or_never().is_none(),
        "disconnector should not resolve before shutdown() completes"
    );

    tx.send(Ok(())).unwrap();
    pool.run_until(disconnector_handle).unwrap();
}

#[test]
fn disconnector_propagates_shutdown_error() {
    let (mut pool, tx, _shutdown_called, disconnector_handle) = mock_setup();

    tx.send(Err(Error::failed("mock shutdown failure".into())))
        .unwrap();
    match pool.run_until(disconnector_handle) {
        Err(e) => assert!(
            e.to_string().contains("mock shutdown failure"),
            "unexpected error: {e:?}"
        ),
        Ok(()) => panic!("disconnector should have reported the shutdown error"),
    }
}
