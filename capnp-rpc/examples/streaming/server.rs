use std::net::ToSocketAddrs;

use crate::streaming_capnp::{byte_stream, receiver};
use capnp_rpc::{pry, rpc_twoparty_capnp, twoparty, RpcSystem};

use capnp::capability::Promise;
use capnp::Error;

use futures::channel::oneshot;
use futures::AsyncReadExt;
use sha2::{Digest, Sha256};

struct ByteStreamImpl {
    hasher: Sha256,
    hash_sender: Option<oneshot::Sender<Vec<u8>>>,
}

impl ByteStreamImpl {
    fn new(hash_sender: oneshot::Sender<Vec<u8>>) -> Self {
        Self {
            hasher: Sha256::new(),
            hash_sender: Some(hash_sender),
        }
    }
}

impl byte_stream::Server for ByteStreamImpl {
    fn write(&mut self, params: byte_stream::WriteParams) -> Promise<(), Error> {
        let bytes = pry!(pry!(params.get()).get_bytes());
        self.hasher.update(bytes);
        Promise::ok(())
    }

    fn end(
        &mut self,
        _params: byte_stream::EndParams,
        _results: byte_stream::EndResults,
    ) -> Promise<(), Error> {
        let hasher = std::mem::take(&mut self.hasher);
        if let Some(sender) = self.hash_sender.take() {
            let _ = sender.send(hasher.finalize()[..].to_vec());
        }
        Promise::ok(())
    }
}

struct ReceiverImpl {}

impl ReceiverImpl {
    fn new() -> Self {
        Self {}
    }
}

impl receiver::Server for ReceiverImpl {
    fn write_stream(
        &mut self,
        _params: receiver::WriteStreamParams,
        mut results: receiver::WriteStreamResults,
    ) -> Promise<(), Error> {
        let (snd, rcv) = oneshot::channel();
        let client: byte_stream::Client = capnp_rpc::new_client(ByteStreamImpl::new(snd));
        results.get().set_stream(client);
        pry!(results.set_pipeline());
        Promise::from_future(async move {
            match rcv.await {
                Ok(v) => {
                    results.get().set_sha256(&v[..]);
                    Ok(())
                }
                Err(_) => Err(Error::failed("failed to get hash".into())),
            }
        })
    }
}

pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {} server ADDRESS[:PORT]", args[0]);
        return Ok(());
    }

    let addr = args[2]
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");

    tokio::task::LocalSet::new()
        .run_until(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            let client: receiver::Client = capnp_rpc::new_client(ReceiverImpl::new());

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
