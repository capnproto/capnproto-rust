// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
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

use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::channel::oneshot;

use crate::foo_capnp::foo;

use futures::{AsyncReadExt, TryFutureExt};

// Rust server defining an implementation of Foo.
struct FooImpl {
    disconnect: RefCell<Option<oneshot::Sender<()>>>,
}
impl FooImpl {
    pub fn new() -> (Self, oneshot::Receiver<()>) {
        let (sender, receiver) = oneshot::channel();
        (
            FooImpl {
                disconnect: RefCell::new(Some(sender)),
            },
            receiver,
        )
    }
}
impl foo::Server for FooImpl {
    async fn identity(
        self: std::rc::Rc<Self>,
        params: foo::IdentityParams,
        mut results: foo::IdentityResults,
    ) -> Result<(), ::capnp::Error> {
        let x = params.get()?.get_x();
        results.get().set_y(x);
        Ok(())
    }

    async fn crash(
        self: std::rc::Rc<Self>,
        _: foo::CrashParams,
        _: foo::CrashResults,
    ) -> Result<(), ::capnp::Error> {
        if let Some(d) = self.disconnect.borrow_mut().take() {
            let _ = d.send(());
        }
        Ok(())
    }
}

pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {} server ADDRESS[:PORT]", args[0]);
        return Ok(());
    }
    tokio::task::LocalSet::new().run_until(try_main(args)).await
}

async fn try_main(args: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    use std::net::ToSocketAddrs;
    let addr = args[2]
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        stream.set_nodelay(true)?;
        let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
        let network = twoparty::VatNetwork::new(
            futures::io::BufReader::new(reader),
            futures::io::BufWriter::new(writer),
            rpc_twoparty_capnp::Side::Server,
            Default::default(),
        );
        let (foo_impl, crashed) = FooImpl::new();
        let foo_client: foo::Client = capnp_rpc::new_client(foo_impl);

        let rpc_system = RpcSystem::new(Box::new(network), Some(foo_client.clone().client));
        let disconnector = rpc_system.get_disconnector();
        tokio::task::spawn_local(async move {
            if crashed.await.is_ok() {
                eprintln!("We were told to crash!")
            } else {
                eprintln!("Shutting down");
            }
            if let Err(err) = disconnector.await {
                eprintln!("error shutting down: {err:#?}");
            }
        });
        tokio::task::spawn_local(rpc_system.map_err(|e| eprintln!("error: {e:?}")));
    }
}
