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

use crate::{echo_capnp::echo, shared_secret_capnp::shared_secret_authenticated};
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};

use futures::AsyncReadExt;

pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::net::ToSocketAddrs;
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 5 {
        println!("usage: {} client HOST:PORT SECRET MESSAGE", args[0]);
        return Ok(());
    }

    let addr = &args[2]
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");

    let secret = &args[3];
    let message = &args[4];

    tokio::task::LocalSet::new()
        .run_until(async move {
            let stream = tokio::net::TcpStream::connect(&addr).await?;
            stream.set_nodelay(true)?;
            let (reader, writer) =
                tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
            let rpc_network = Box::new(twoparty::VatNetwork::new(
                futures::io::BufReader::new(reader),
                futures::io::BufWriter::new(writer),
                rpc_twoparty_capnp::Side::Client,
                Default::default(),
            ));

            let mut rpc_system = RpcSystem::new(rpc_network, None);
            let shared_secret_client: shared_secret_authenticated::Client<echo::Owned> =
                rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

            tokio::task::spawn_local(rpc_system);
            let mut request = shared_secret_client.authenticate_request();
            request.get().set_shared_secret(secret);
            let authed_echo = request.send().pipeline.get_authenticated();
            let mut echo_request = authed_echo.echo_request();
            echo_request.get().set_message(message);
            let resp = echo_request.send().promise.await?;
            let msg = resp.get()?.get_response_message()?.to_str()?;
            println!("Received message: {}", msg);

            Ok(())
        })
        .await
}
