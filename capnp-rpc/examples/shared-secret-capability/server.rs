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

use std::{
    net::ToSocketAddrs, rc::Rc,
};

use crate::{echo_capnp::echo::{EchoParams, EchoResults}, shared_secret_capnp::{shared_secret_authenticated::{self, AuthenticateParams, AuthenticateResults}}};
use crate::echo_capnp::echo;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};

use capnp::{Error, traits::{Owned, SetterInput}};

use futures::AsyncReadExt;

struct SharedSecretImpl<C> {
    secret: String,
    inner_cap: C
}


impl<T: Owned + 'static, C: SetterInput<T> + Clone + 'static> shared_secret_authenticated::Server<T> for SharedSecretImpl<C> {
    async fn authenticate(self: Rc<Self>,
        params: AuthenticateParams<T>,
        mut results: AuthenticateResults<T>) -> Result<(), Error>
    {
        let secret = params.get()?
            .get_shared_secret()?
            .to_str()?;
        if secret != self.secret {
            return Err(Error::failed("Auth failed".to_owned()));
        }

        results
            .get()
            .set_authenticated(self.inner_cap.clone())?;
        Ok(())
    }
}

struct EchoImpl {}

impl echo::Server for EchoImpl {
    async fn echo(self: Rc<Self>, params: EchoParams, mut results: EchoResults<>) -> Result<(), Error> {
        let msg = params.get()?.get_message()?;
        let mut response = results.get();
        response.set_response_message(msg);
        Ok(())
    }
}


pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 4 {
        println!("usage: {} server ADDRESS[:PORT] SECRET", args[0]);
        return Ok(());
    }

    let addr = &args[2]
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");
    let secret = &args[3];

    tokio::task::LocalSet::new()
        .run_until(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await?;

            let echo_impl = EchoImpl{};
            let echo: echo::Client = capnp_rpc::new_client(echo_impl);

            let shared_secret_impl = SharedSecretImpl{
                secret: secret.to_owned(),
                inner_cap: echo
            };
            let client: shared_secret_authenticated::Client<echo::Owned> = capnp_rpc::new_client(shared_secret_impl);

            loop {
                let (stream, _) = listener.accept().await?;
                stream.set_nodelay(true)?;
                let (reader, writer) =
                    tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
                let network = twoparty::VatNetwork::new(
                    futures::io::BufReader::new(reader),
                    futures::io::BufWriter::new(writer),
                    rpc_twoparty_capnp::Side::Server,
                    Default::default(),
                );

                let rpc_system = RpcSystem::new(Box::new(network), Some(client.clone().client));

                tokio::task::spawn_local(rpc_system);
            }
        })
        .await
}
