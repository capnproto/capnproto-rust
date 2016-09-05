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
use pubsub_capnp::{publisher, subscriber, subscription};

use gj::{EventLoop, Promise, TaskReaper, TaskSet};

struct SubscriberHandle {
    client: subscriber::Client<::capnp::text::Owned>,
    requests_in_flight: i32,
}

struct SubscriberMap {
    subscribers: HashMap<u64, SubscriberHandle>,
}

impl SubscriberMap {
    fn new() -> SubscriberMap {
        SubscriberMap { subscribers: HashMap::new() }
    }
}

struct SubscriptionImpl {
    id: u64,
    subscribers: Rc<RefCell<SubscriberMap>>,
}

impl SubscriptionImpl {
    fn new(id: u64, subscribers: Rc<RefCell<SubscriberMap>>) -> SubscriptionImpl {
        SubscriptionImpl { id: id, subscribers: subscribers }
    }
}

impl Drop for SubscriptionImpl {
    fn drop(&mut self) {
        println!("subscription dropped");
        self.subscribers.borrow_mut().subscribers.remove(&self.id);
    }
}

impl subscription::Server for SubscriptionImpl {}

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

impl publisher::Server<::capnp::text::Owned> for PublisherImpl {
    fn subscribe(&mut self,
                 params: publisher::SubscribeParams<::capnp::text::Owned>,
                 mut results: publisher::SubscribeResults<::capnp::text::Owned>,)
                 -> Promise<(), ::capnp::Error>
    {
        println!("subscribe");
        self.subscribers.borrow_mut().subscribers.insert(
            self.next_id,
            SubscriberHandle {
                client: pry!(pry!(params.get()).get_subscriber()),
                requests_in_flight: 0,
            }
        );

        results.get().set_subscription(
            subscription::ToClient::new(SubscriptionImpl::new(self.next_id, self.subscribers.clone()))
                .from_server::<::capnp_rpc::Server>());

        self.next_id += 1;
        Promise::ok(())
    }
}

pub fn accept_loop(listener: ::gjio::SocketListener,
                   task_set: Rc<RefCell<TaskSet<(), ::capnp::Error>>>,
                   publisher: publisher::Client<::capnp::text::Owned>)
                   -> Promise<(), ::std::io::Error>
{
    listener.accept().then(move |stream| {
        let mut network =
            twoparty::VatNetwork::new(stream.clone(), stream,
                                      rpc_twoparty_capnp::Side::Server, Default::default());
        let disconnect_promise = network.on_disconnect();

        let rpc_system = RpcSystem::new(Box::new(network), Some(publisher.clone().client));

        task_set.borrow_mut().add(disconnect_promise.attach(rpc_system));
        accept_loop(listener, task_set, publisher)
    })
}

struct Reaper;

impl TaskReaper<(), ::capnp::Error> for Reaper {
    fn task_failed(&mut self, error: ::capnp::Error) {
        println!("Task failed: {}", error);
    }
}

fn send_to_subscribers(subscribers: Rc<RefCell<SubscriberMap>>,
                       timer: ::gjio::Timer,
                       task_set: Rc<RefCell<TaskSet<(), ::capnp::Error>>>)
                       -> Promise<(), ::capnp::Error>
{
    timer.after_delay(::std::time::Duration::new(1, 0)).lift().then(move |()| {
        {
            let subscribers1 = subscribers.clone();
            let subs = &mut subscribers.borrow_mut().subscribers;
            for (&idx, mut subscriber) in subs.iter_mut() {
                if subscriber.requests_in_flight < 5 {
                    subscriber.requests_in_flight += 1;
                    let mut request = subscriber.client.push_message_request();
                    pry!(request.get().set_message(
                        &format!("system time is: {:?}", ::std::time::SystemTime::now())[..]));

                    let subscribers2 = subscribers1.clone();
                    task_set.borrow_mut().add(request.send().promise.map_else(move |r| {
                        match r {
                            Ok(_) => {
                                subscribers2.borrow_mut().subscribers.get_mut(&idx).map(|ref mut s| {
                                    s.requests_in_flight -= 1;
                                });
                            }
                            Err(e) => {
                                println!("Got error: {:?}. Dropping subscriber.", e);
                                subscribers2.borrow_mut().subscribers.remove(&idx);
                            }
                        }
                        Ok(())
                    }));
                }
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
