use std::cell::RefCell;
use std::rc::Rc;

use capnp::capability::Promise;
use futures::channel::oneshot;
use futures::FutureExt;

struct FailedNetwork {
    accept: Option<Promise<Box<dyn capnp_rpc::Connection<()>>, capnp::Error>>,
}

impl capnp_rpc::VatNetwork<()> for FailedNetwork {
    fn connect(&mut self, _: ()) -> Option<Box<dyn capnp_rpc::Connection<()>>> {
        None
    }

    fn accept(&mut self) -> Promise<Box<dyn capnp_rpc::Connection<()>>, capnp::Error> {
        self.accept.take().unwrap()
    }

    fn drive_until_shutdown(&mut self) -> Promise<(), capnp::Error> {
        Promise::from_future(std::future::pending())
    }
}

#[test]
fn background_errors_use_custom_handler() {
    let errors = Rc::new(RefCell::new(Vec::new()));
    let captured_errors = errors.clone();
    let network = FailedNetwork {
        accept: Some(Promise::err(capnp::Error::failed(
            "background task failed".into(),
        ))),
    };
    let mut rpc_system = capnp_rpc::RpcSystem::new(Box::new(network), None);
    rpc_system.set_background_error_handler(move |error| {
        captured_errors.borrow_mut().push(error);
    });

    assert!((&mut rpc_system).now_or_never().is_none());
    let errors = errors.borrow();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, capnp::ErrorKind::Failed);
    assert_eq!(errors[0].extra, "background task failed");
    assert!((&mut rpc_system).now_or_never().is_none());
}

#[test]
fn replacement_handler_applies_to_running_tasks() {
    let (sender, receiver) = oneshot::channel();
    let network = FailedNetwork {
        accept: Some(Promise::from_future(
            async move { Err(receiver.await.unwrap()) },
        )),
    };
    let mut rpc_system = capnp_rpc::RpcSystem::new(Box::new(network), None);
    rpc_system.set_background_error_handler(|_| panic!("old handler must not be called"));
    assert!((&mut rpc_system).now_or_never().is_none());

    let errors = Rc::new(RefCell::new(Vec::new()));
    let captured_errors = errors.clone();
    rpc_system.set_background_error_handler(move |error| {
        captured_errors.borrow_mut().push(error);
    });
    sender
        .send(capnp::Error::failed("delayed failure".into()))
        .unwrap();

    assert!((&mut rpc_system).now_or_never().is_none());
    let errors = errors.borrow();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].extra, "delayed failure");
}
