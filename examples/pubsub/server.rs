extern crate capnp;
extern crate capnp_rpc;

#[macro_use]
extern crate gj;

use std::rc::Rc;
use std::cell::RefCell;

use capnp_rpc::{RpcSystem, twoparty, rpc_twoparty_capnp};
use pubsub_capnp::{publisher, subscriber};

use gj::{EventLoop, Promise, TaskReaper, TaskSet};
use gj::io::tcp;

pub mod pubsub_capnp {
  include!(concat!(env!("OUT_DIR"), "/pubsub_capnp.rs"));
}

struct PublisherImpl {
  subscribers: Rc<RefCell<Vec<subscriber::Client>>>,
}

impl PublisherImpl {
    pub fn new() -> (PublisherImpl, Rc<RefCell<Vec<subscriber::Client>>>) {
        let subscribers = Rc::new(RefCell::new(Vec::new()));
        (PublisherImpl { subscribers: subscribers.clone() }, subscribers.clone())
    }
}

impl publisher::Server for PublisherImpl {
    fn register(&mut self,
                params: publisher::RegisterParams,
                _results: publisher::RegisterResults,)
                -> Promise<(), capnp::Error>
    {
        println!("Register");
        self.subscribers.borrow_mut().push(pry!(pry!(params.get()).get_subscriber()));
        Promise::ok(())
    }
}

pub fn accept_loop(listener: tcp::Listener,
                   task_set: Rc<RefCell<TaskSet<(), Box<::std::error::Error>>>>,
                   publisher: publisher::Client)
                   -> Promise<(), ::std::io::Error>
{
    listener.accept().lift().then(move |(listener, stream)| {
        let (reader, writer) = stream.split();
        let mut network =
            twoparty::VatNetwork::new(reader, writer,
                                      rpc_twoparty_capnp::Side::Server, Default::default());
        let disconnect_promise = network.on_disconnect();

        let rpc_system = RpcSystem::new(Box::new(network), Some(publisher.clone().client));

        task_set.borrow_mut().add(disconnect_promise.attach(rpc_system).lift());
        accept_loop(listener, task_set, publisher)
    })
}

struct Reaper;

impl TaskReaper<(), Box<::std::error::Error>> for Reaper {
    fn task_failed(&mut self, error: Box<::std::error::Error>) {
        println!("Task failed: {}", error);
    }
}


fn send_to_subscribers(subscribers: Rc<RefCell<Vec<subscriber::Client>>>,
                       task_set: Rc<RefCell<TaskSet<(), Box<::std::error::Error>>>>)
                       -> Promise<(), Box<::std::error::Error>>
{
    gj::io::Timer.after_delay(::std::time::Duration::new(1, 0)).lift().then(move |()| {
        {
            for subscriber in subscribers.borrow().iter() {
                let mut request = subscriber.push_values_request();
                request.get().set_values(1.23);
                task_set.borrow_mut().add(request.send().promise.map(|_| Ok(())).lift());
            }
        }
        send_to_subscribers(subscribers, task_set)
    })
}

pub fn main() {
    EventLoop::top_level(move |wait_scope| {
        use std::net::ToSocketAddrs;
        let addr = try!("127.0.0.1:22222".to_socket_addrs()).next().expect("could not parse address");
        let listener = try!(tcp::Listener::bind(addr));

        let (publisher_impl, subscribers) = PublisherImpl::new();

        let publisher = publisher::ToClient::new(publisher_impl).from_server::<::capnp_rpc::Server>();

        let task_set = Rc::new(RefCell::new(TaskSet::new(Box::new(Reaper))));

        let task_set_clone = task_set.clone();

        task_set.borrow_mut().add(send_to_subscribers(subscribers, task_set_clone));

        try!(accept_loop(listener, task_set, publisher).wait(wait_scope));

        Ok(())
    }).expect("top level error");
}
