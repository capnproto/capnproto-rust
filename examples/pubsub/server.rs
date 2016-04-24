// Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
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

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

use capnp_rpc::{RpcSystem, twoparty, rpc_twoparty_capnp};
use pubsub_capnp::{publisher, subscriber, handle};

use gj::{EventLoop, Promise, TaskReaper, TaskSet};

struct SubscriberMap {
    subscribers: HashMap<u64, subscriber::Client>,
}

impl SubscriberMap {
    fn new() -> SubscriberMap {
        SubscriberMap { subscribers: HashMap::new() }
    }
}

struct HandleImpl {
    id: u64,
    subscribers: Rc<RefCell<SubscriberMap>>,
}

impl HandleImpl {
    fn new(id: u64, subscribers: Rc<RefCell<SubscriberMap>>) -> HandleImpl {
        HandleImpl { id: id, subscribers: subscribers }
    }
}

impl Drop for HandleImpl {
    fn drop(&mut self) {
        self.subscribers.borrow_mut().subscribers.remove(&self.id);
    }
}

impl handle::Server for HandleImpl {}

struct PublisherImpl {
    next_id: u64,
    subscribers: Rc<RefCell<SubscriberMap>>,
}

impl PublisherImpl {
    pub fn new() -> (PublisherImpl, Rc<RefCell<SubscriberMap>>) {
        let subscribers = Rc::new(RefCell::new(SubscriberMap::new()));
        (PublisherImpl { next_id: 0, subscribers: subscribers.clone() },
         subscribers.clone())
    }
}

impl publisher::Server for PublisherImpl {
    fn register(&mut self,
                params: publisher::RegisterParams,
                mut results: publisher::RegisterResults,)
                -> Promise<(), ::capnp::Error>
    {
        println!("Register");
        self.subscribers.borrow_mut().subscribers.insert(self.next_id,
                                                         pry!(pry!(params.get()).get_subscriber()));

        results.get().set_handle(
            handle::ToClient::new(HandleImpl::new(self.next_id, self.subscribers.clone()))
                .from_server::<::capnp_rpc::Server>());

        self.next_id += 1;
        Promise::ok(())
    }
}

pub fn accept_loop(mut listener: ::gjio::SocketListener,
                   task_set: Rc<RefCell<TaskSet<(), Box<::std::error::Error>>>>,
                   publisher: publisher::Client)
                   -> Promise<(), ::std::io::Error>
{
    listener.accept().then(move |stream| {
        let mut network =
            twoparty::VatNetwork::new(stream.clone(), stream,
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

fn send_to_subscribers(subscribers: Rc<RefCell<SubscriberMap>>,
                       timer: ::gjio::Timer,
                       task_set: Rc<RefCell<TaskSet<(), Box<::std::error::Error>>>>)
                       -> Promise<(), Box<::std::error::Error>>
{
    timer.after_delay(::std::time::Duration::new(1, 0)).lift().then(move |()| {
        {
            for (_, subscriber) in subscribers.borrow().subscribers.iter() {
                let mut request = subscriber.push_values_request();
                request.get().set_values(1.23);
                task_set.borrow_mut().add(request.send().promise.map(|_| Ok(())).lift());
            }
        }
        send_to_subscribers(subscribers, timer, task_set)
    })
}

pub fn main() {
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {} server HOST:PORT", args[0]);
        return;
    }

    EventLoop::top_level(move |wait_scope| -> Result<(), Box<::std::error::Error>> {
        use std::net::ToSocketAddrs;
        let mut event_port = try!(::gjio::EventPort::new());
        let network = event_port.get_network();
        let addr = try!(args[2].to_socket_addrs()).next().expect("could not parse address");
        let mut address = network.get_tcp_address(addr);
        let listener = try!(address.listen());

        let (publisher_impl, subscribers) = PublisherImpl::new();

        let publisher = publisher::ToClient::new(publisher_impl).from_server::<::capnp_rpc::Server>();

        let task_set = Rc::new(RefCell::new(TaskSet::new(Box::new(Reaper))));

        let task_set_clone = task_set.clone();

        task_set.borrow_mut().add(send_to_subscribers(subscribers, event_port.get_timer(), task_set_clone));

        try!(accept_loop(listener, task_set, publisher).wait(wait_scope, &mut event_port));

        Ok(())
    }).expect("top level error");
}
