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

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::pubsub_capnp::{publisher, subscriber, subscription};
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};

use capnp::capability::Promise;

use futures::{AsyncReadExt, FutureExt, StreamExt};

struct SubscriberHandle {
    client: subscriber::Client<::capnp::text::Owned>,
    requests_in_flight: i32,
}

struct SubscriberMap {
    subscribers: HashMap<u64, SubscriberHandle>,
}

impl SubscriberMap {
    fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
        }
    }
}

struct SubscriptionImpl {
    id: u64,
    subscribers: Rc<RefCell<SubscriberMap>>,
}

impl SubscriptionImpl {
    fn new(id: u64, subscribers: Rc<RefCell<SubscriberMap>>) -> Self {
        Self { id, subscribers }
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
    pub fn new() -> (Self, Rc<RefCell<SubscriberMap>>) {
        let subscribers = Rc::new(RefCell::new(SubscriberMap::new()));
        (
            Self {
                next_id: 0,
                subscribers: subscribers.clone(),
            },
            subscribers,
        )
    }
}

impl publisher::Server<::capnp::text::Owned> for PublisherImpl {
    fn subscribe(
        &mut self,
        params: publisher::SubscribeParams<::capnp::text::Owned>,
        mut results: publisher::SubscribeResults<::capnp::text::Owned>,
    ) -> Promise<(), ::capnp::Error> {
        println!("subscribe");
        self.subscribers.borrow_mut().subscribers.insert(
            self.next_id,
            SubscriberHandle {
                client: pry!(pry!(params.get()).get_subscriber()),
                requests_in_flight: 0,
            },
        );

        results
            .get()
            .set_subscription(capnp_rpc::new_client(SubscriptionImpl::new(
                self.next_id,
                self.subscribers.clone(),
            )));

        self.next_id += 1;
        Promise::ok(())
    }
}

pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::net::ToSocketAddrs;
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {} server HOST:PORT", args[0]);
        return Ok(());
    }

    let addr = args[2]
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");

    tokio::task::LocalSet::new()
        .run_until(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            let (publisher_impl, subscribers) = PublisherImpl::new();
            let publisher: publisher::Client<_> = capnp_rpc::new_client(publisher_impl);

            let handle_incoming = async move {
                loop {
                    let (stream, _) = listener.accept().await?;
                    stream.set_nodelay(true)?;
                    let (reader, writer) =
                        tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
                    let network = twoparty::VatNetwork::new(
                        reader,
                        writer,
                        rpc_twoparty_capnp::Side::Server,
                        Default::default(),
                    );
                    let rpc_system =
                        RpcSystem::new(Box::new(network), Some(publisher.clone().client));

                    tokio::task::spawn_local(rpc_system);
                }
            };

            // Trigger sending approximately once per second.
            let (tx, mut rx) = futures::channel::mpsc::unbounded::<()>();
            std::thread::spawn(move || {
                while let Ok(()) = tx.unbounded_send(()) {
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
            });

            let send_to_subscribers = async move {
                while let Some(()) = rx.next().await {
                    let subscribers1 = subscribers.clone();
                    let subs = &mut subscribers.borrow_mut().subscribers;
                    for (&idx, mut subscriber) in subs.iter_mut() {
                        if subscriber.requests_in_flight < 5 {
                            subscriber.requests_in_flight += 1;
                            let mut request = subscriber.client.push_message_request();
                            request.get().set_message(
                            &format!("system time is: {:?}", ::std::time::SystemTime::now())[..])?;
                            let subscribers2 = subscribers1.clone();
                            tokio::task::spawn_local(request.send().promise.map(
                                move |r| match r {
                                    Ok(_) => {
                                        if let Some(ref mut s) =
                                            subscribers2.borrow_mut().subscribers.get_mut(&idx)
                                        {
                                            s.requests_in_flight -= 1;
                                        }
                                    }
                                    Err(e) => {
                                        println!("Got error: {e:?}. Dropping subscriber.");
                                        subscribers2.borrow_mut().subscribers.remove(&idx);
                                    }
                                },
                            ));
                        }
                    }
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            };

            let _: ((), ()) =
                futures::future::try_join(handle_incoming, send_to_subscribers).await?;
            Ok(())
        })
        .await
}
