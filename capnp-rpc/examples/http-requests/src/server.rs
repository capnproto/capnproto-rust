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

use capnp_rpc::{RpcSystem, twoparty, rpc_twoparty_capnp};
use crate::http_capnp::{outgoing_http, http_session};

use capnp::capability::Promise;
use capnp::Error;

use futures::{AsyncReadExt, FutureExt, StreamExt, TryFutureExt};
use tokio_util::compat::Tokio02AsyncReadCompatExt;

struct OutgoingHttp;

impl OutgoingHttp {
    fn new() -> OutgoingHttp {
        OutgoingHttp
    }
}

impl outgoing_http::Server for OutgoingHttp {
    fn new_session(
        &mut self,
        params: outgoing_http::NewSessionParams,
        mut results: outgoing_http::NewSessionResults)
        -> Promise<(), Error>
    {
        let session = HttpSession::new(
            pry!(pry!(params.get()).get_base_url()).to_string());
        results.get().set_session(
            http_session::ToClient::new(session).into_client::<::capnp_rpc::Server>());
        Promise::ok(())
    }
}

struct HttpSession {
    base_url: String,
}

impl HttpSession {
    fn new(base_url: String) -> HttpSession {
        HttpSession {
            base_url: base_url,
        }
    }
}

impl http_session::Server for HttpSession {
    fn get(
        &mut self,
        params: http_session::GetParams,
        mut results: http_session::GetResults)
        -> Promise<(), Error>
    {
        let path = pry!(pry!(params.get()).get_path());
        let mut url = self.base_url.clone();
        url.push_str(path);
        let url = url.parse::<hyper::Uri>().unwrap();
        let client = hyper::Client::new();
        Promise::from_future(async move {
            let res = client.get(url).await?;
            results.get().set_response_code(res.status().as_u16() as u32);
            let mut body = res.into_body();
            let mut body_bytes: Vec<u8> = Vec::new();
            while let Some(next) = body.next().await {
                let chunk = next?;
                std::io::Write::write_all(&mut body_bytes, &chunk[..])?;
            }
            results.get().set_body(&body_bytes[..]);
            Ok(())
        }.map_err(|e: Box<dyn std::error::Error>| Error::failed(format!("{:?}", e))))
    }

    fn post(
        &mut self,
        _params: http_session::PostParams,
        _results: http_session::PostResults)
        -> Promise<(), Error>
    {
        // TODO
        Promise::err(Error::unimplemented(format!("post() is unimplemented")))
    }
}

pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::net::ToSocketAddrs;
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {} server HOST:PORT", args[0]);
        return Ok(());
    }
    let addr = args[2].to_socket_addrs().unwrap().next().expect("could not parse address");
    let mut listener = tokio::net::TcpListener::bind(&addr).await?;
    let proxy = outgoing_http::ToClient::new(OutgoingHttp::new()).into_client::<::capnp_rpc::Server>();
    let local = tokio::task::LocalSet::new();
    local.run_until(async {
        loop {
            let (stream, _) = listener.accept().await?;
            stream.set_nodelay(true)?;
            let (reader, writer) = stream.compat().split();

            let network =
                twoparty::VatNetwork::new(reader, writer,
                                          rpc_twoparty_capnp::Side::Server, Default::default());

            let rpc_system = RpcSystem::new(Box::new(network), Some(proxy.clone().client));

            tokio::task::spawn_local(Box::pin(rpc_system.map(|_| ())));
        }
    }).await
}
