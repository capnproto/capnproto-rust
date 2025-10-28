use std::{
    cell::{Cell, RefCell},
    net::ToSocketAddrs,
};

use crate::streaming_capnp::{byte_stream, receiver};
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};

use capnp::Error;

use futures::channel::oneshot;
use futures::AsyncReadExt;
use sha2::{Digest, Sha256};

struct ByteStreamImpl {
    hasher: RefCell<Sha256>,
    bytes_received: Cell<u32>,
    hash_sender: RefCell<Option<oneshot::Sender<Vec<u8>>>>,
}

impl ByteStreamImpl {
    fn new(hash_sender: oneshot::Sender<Vec<u8>>) -> Self {
        Self {
            hasher: RefCell::new(Sha256::new()),
            bytes_received: Cell::new(0),
            hash_sender: RefCell::new(Some(hash_sender)),
        }
    }
}

impl byte_stream::Server for ByteStreamImpl {
    async fn write(self: std::rc::Rc<Self>, params: byte_stream::WriteParams) -> Result<(), Error> {
        let bytes = params.get()?.get_bytes()?;
        self.hasher.borrow_mut().update(bytes);
        self.bytes_received
            .set(self.bytes_received.get() + bytes.len() as u32);
        Ok(())
    }

    async fn end(
        self: std::rc::Rc<Self>,
        _params: byte_stream::EndParams,
        _results: byte_stream::EndResults,
    ) -> Result<(), Error> {
        let hasher = std::mem::take(&mut *self.hasher.borrow_mut());
        let hash = hasher.finalize()[..].to_vec();
        println!(
            "received {} bytes with hash {}",
            self.bytes_received.get(),
            base16::encode_lower(&hash[..])
        );
        if let Some(sender) = self.hash_sender.borrow_mut().take() {
            let _ = sender.send(hash);
        }
        Ok(())
    }
}

struct ReceiverImpl {}

impl ReceiverImpl {
    fn new() -> Self {
        Self {}
    }
}

impl receiver::Server for ReceiverImpl {
    async fn write_stream(
        self: std::rc::Rc<Self>,
        _params: receiver::WriteStreamParams,
        mut results: receiver::WriteStreamResults,
    ) -> Result<(), Error> {
        let (snd, rcv) = oneshot::channel();
        let client: byte_stream::Client = capnp_rpc::new_client(ByteStreamImpl::new(snd));
        results.get().set_stream(client);
        results.set_pipeline()?;

        match rcv.await {
            Ok(v) => {
                results.get().set_sha256(&v[..]);
                Ok(())
            }
            Err(_) => Err(Error::failed("failed to get hash".into())),
        }
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
