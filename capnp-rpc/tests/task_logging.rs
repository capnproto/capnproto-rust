use std::sync::Mutex;

use capnp::capability::Promise;
use futures::FutureExt;

struct TestLogger(Mutex<Vec<(log::Level, String, String)>>);

impl log::Log for TestLogger {
    fn enabled(&self, _: &log::Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &log::Record<'_>) {
        self.0.lock().unwrap().push((
            record.level(),
            record.target().to_owned(),
            record.args().to_string(),
        ));
    }

    fn flush(&self) {}
}

static LOGGER: TestLogger = TestLogger(Mutex::new(Vec::new()));

struct FailedNetwork;

impl capnp_rpc::VatNetwork<()> for FailedNetwork {
    fn connect(&mut self, _: ()) -> Option<Box<dyn capnp_rpc::Connection<()>>> {
        None
    }

    fn accept(&mut self) -> Promise<Box<dyn capnp_rpc::Connection<()>>, capnp::Error> {
        Promise::err(capnp::Error::failed("background task failed".into()))
    }

    fn drive_until_shutdown(&mut self) -> Promise<(), capnp::Error> {
        Promise::from_future(std::future::pending())
    }
}

#[test]
fn background_errors_use_the_application_logger() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(log::LevelFilter::Error);

    let rpc_system = capnp_rpc::RpcSystem::new(Box::new(FailedNetwork), None);
    assert!(rpc_system.now_or_never().is_none());
    assert_eq!(
        *LOGGER.0.lock().unwrap(),
        vec![(
            log::Level::Error,
            "capnp_rpc".to_owned(),
            "Failed: background task failed".to_owned(),
        )]
    );
}
