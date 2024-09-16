use crate::streaming_capnp::receiver;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};

use futures::AsyncReadExt;
use rand::Rng;
use sha2::{Digest, Sha256};

pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::net::ToSocketAddrs;
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 5 {
        println!(
            "usage: {} client HOST:PORT STREAM_SIZE WINDOW_SIZE",
            args[0]
        );
        return Ok(());
    }

    let stream_size: usize = str::parse(&args[3]).unwrap();
    let window_size: usize = str::parse(&args[4]).unwrap();

    let addr = args[2]
        .to_socket_addrs()?
        .next()
        .expect("could not parse address");

    tokio::task::LocalSet::new()
        .run_until(async move {
            let stream = tokio::net::TcpStream::connect(&addr).await?;
            stream.set_nodelay(true)?;
            let (reader, writer) =
                tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
            let mut rpc_network = Box::new(twoparty::VatNetwork::new(
                futures::io::BufReader::new(reader),
                futures::io::BufWriter::new(writer),
                rpc_twoparty_capnp::Side::Client,
                Default::default(),
            ));
            rpc_network.set_window_size(window_size);
            let mut rpc_system = RpcSystem::new(rpc_network, None);
            let receiver: receiver::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
            tokio::task::spawn_local(rpc_system);

            let capnp::capability::RemotePromise { promise, pipeline } =
                receiver.write_stream_request().send();

            let mut rng = rand::thread_rng();
            let mut hasher = Sha256::new();
            let bytestream = pipeline.get_stream();
            let mut bytes_written: u32 = 0;
            const CHUNK_SIZE: u32 = 4096;
            while bytes_written < stream_size as u32 {
                let mut request = bytestream.write_request();
                let body = request.get();
                let buf = body.init_bytes(CHUNK_SIZE);
                rng.fill(buf);
                hasher.update(buf);
                request.send().await?;
                bytes_written += CHUNK_SIZE;
            }
            bytestream.end_request().send().promise.await?;
            let response = promise.await?;

            let sha256 = response.get()?.get_sha256()?;
            let local_sha256 = hasher.finalize();
            assert_eq!(sha256, &local_sha256[..]);
            Ok(())
        })
        .await
}
