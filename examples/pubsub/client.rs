extern crate capnp;
extern crate capnp_rpc;

#[macro_use]
extern crate gj;

use capnp_rpc::{RpcSystem, twoparty, rpc_twoparty_capnp};
use pubsub_capnp::{publisher, subscriber};

use gj::{EventLoop, Promise};

pub mod pubsub_capnp {
  include!(concat!(env!("OUT_DIR"), "/pubsub_capnp.rs"));
}

struct SubscriberImpl;

impl subscriber::Server for SubscriberImpl {
    fn push_values(&mut self,
                   params: subscriber::PushValuesParams,
                   _results: subscriber::PushValuesResults)
        -> Promise<(), ::capnp::Error>
    {
        println!("got: {}", pry!(params.get()).get_values());
        Promise::ok(())
    }
}

pub fn main() {
    EventLoop::top_level(move |wait_scope| {
        use std::net::ToSocketAddrs;
        let addr = try!("127.0.0.1:22222".to_socket_addrs()).next().expect("could not parse address");
        let (reader, writer) = try!(::gj::io::tcp::Stream::connect(addr).wait(wait_scope)).split();
        let network =
            Box::new(twoparty::VatNetwork::new(reader, writer,
                                               rpc_twoparty_capnp::Side::Client,
                                               Default::default()));
        let mut rpc_system = RpcSystem::new(network, None);
        let publisher: publisher::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

        let sub = subscriber::ToClient::new(SubscriberImpl).from_server::<::capnp_rpc::Server>();

        let mut request = publisher.register_request();
        request.get().set_subscriber(sub);
        request.send().promise.wait(wait_scope).unwrap();

        Promise::<(),()>::never_done().wait(wait_scope).unwrap();
        Ok(())

    }).expect("top level error");
}
