use std::rc::Rc;
use std::sync::mpsc;

use capnp::Error;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::channel::oneshot;
use futures::lock::Mutex;
use futures::AsyncReadExt;
use rand::Rng;
use sha2::{Digest, Sha256};

capnp::generated_code!(pub mod bytestream_capnp);

use bytestream_capnp::{byte_stream, sender, transfer};

// Small chunks let the default 64 KiB flow-control window hold many writes in
// flight, so the race reproduces without increasing VatNetwork's window size.
const STREAM_SIZE: u64 = 1024 * 1024;
const CHUNK_SIZE: u32 = 256;
const WINDOW_SIZE: usize = 1024 * 1024;
// Without an async boundary in `write()`, writes complete synchronously and the
// following `end()` has no pending work to overtake.
const WRITE_YIELDS: usize = 8;

struct ByteStreamImpl {
    state: Rc<Mutex<ByteStreamState>>,
}

struct ByteStreamState {
    hasher: Sha256,
    bytes_received: u64,
    done: Option<oneshot::Sender<(Vec<u8>, u64)>>,
}

impl ByteStreamImpl {
    fn new(done: oneshot::Sender<(Vec<u8>, u64)>) -> Self {
        Self {
            state: Rc::new(Mutex::new(ByteStreamState {
                hasher: Sha256::new(),
                bytes_received: 0,
                done: Some(done),
            })),
        }
    }
}

impl byte_stream::Server for ByteStreamImpl {
    async fn write(self: Rc<Self>, params: byte_stream::WriteParams) -> Result<(), Error> {
        let mut state = self.state.lock().await;

        let bytes = params.get()?.get_bytes()?.to_vec();

        // Model an AsyncWrite sink that returns Pending before completing the write.
        for _ in 0..WRITE_YIELDS {
            tokio::task::yield_now().await;
        }

        state.bytes_received += bytes.len() as u64;
        state.hasher.update(&bytes);
        Ok(())
    }

    async fn end(
        self: Rc<Self>,
        _params: byte_stream::EndParams,
        _results: byte_stream::EndResults,
    ) -> Result<(), Error> {
        let mut state = self.state.lock().await;
        let hash = std::mem::take(&mut state.hasher).finalize()[..].to_vec();
        let bytes_received = state.bytes_received;
        if let Some(done) = state.done.take() {
            let _ = done.send((hash, bytes_received));
        }
        Ok(())
    }
}

struct SenderImpl;

struct TransferImpl {
    state: Rc<Mutex<TransferState>>,
}

struct TransferState {
    result: Option<Result<Vec<u8>, Error>>,
    waiters: Vec<oneshot::Sender<Result<Vec<u8>, Error>>>,
}

impl TransferImpl {
    fn new() -> Self {
        Self {
            state: Rc::new(Mutex::new(TransferState {
                result: None,
                waiters: Vec::new(),
            })),
        }
    }
}

async fn send_bytes(
    stream: byte_stream::Client,
    stream_size: u64,
    chunk_size: u32,
) -> Result<Vec<u8>, Error> {
    let mut rng = rand::rng();
    let mut hasher = Sha256::new();
    let mut bytes_written = 0;
    while bytes_written < stream_size {
        let mut request = stream.write_request();
        let this_chunk_size = u64::min(chunk_size as u64, stream_size - bytes_written);
        let bytes = request.get().init_bytes(this_chunk_size as u32);
        rng.fill(bytes);
        hasher.update(bytes);
        request.send().await?;
        bytes_written += this_chunk_size;
    }

    stream.end_request().send().promise.await?;
    Ok(hasher.finalize()[..].to_vec())
}

async fn finish_send(state: Rc<Mutex<TransferState>>, result: Result<Vec<u8>, Error>) {
    let waiters = {
        let mut state = state.lock().await;
        state.result = Some(result.clone());
        std::mem::take(&mut state.waiters)
    };

    for waiter in waiters {
        let _ = waiter.send(result.clone());
    }
}

impl transfer::Server for TransferImpl {
    async fn wait(
        self: Rc<Self>,
        _params: transfer::WaitParams,
        mut results: transfer::WaitResults,
    ) -> Result<(), Error> {
        let receiver = {
            let mut state = self.state.lock().await;
            if let Some(result) = state.result.clone() {
                match result {
                    Ok(sha256) => {
                        results.get().set_sha256(&sha256[..]);
                        return Ok(());
                    }
                    Err(e) => return Err(e),
                }
            }

            let (sender, receiver) = oneshot::channel();
            state.waiters.push(sender);
            receiver
        };

        let sha256 = match receiver.await {
            Ok(Ok(sha256)) => sha256,
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(Error::failed("send task was canceled".into())),
        };
        results.get().set_sha256(&sha256[..]);
        Ok(())
    }
}

impl sender::Server for SenderImpl {
    async fn send(
        self: Rc<Self>,
        params: sender::SendParams,
        mut results: sender::SendResults,
    ) -> Result<(), Error> {
        let params = params.get()?;
        let stream = params.get_stream()?;
        let stream_size = params.get_size();
        let chunk_size = params.get_chunk_size();

        let transfer_impl = TransferImpl::new();
        let state = transfer_impl.state.clone();
        results
            .get()
            .set_transfer(capnp_rpc::new_client(transfer_impl));

        tokio::task::spawn_local(async move {
            let result = send_bytes(stream, stream_size, chunk_size).await;
            finish_send(state, result).await;
        });
        Ok(())
    }
}

#[test]
fn tcp_bytestream_end_waits_for_streaming_writes() {
    let (addr_sender, addr_receiver) = mpsc::channel();
    let server_thread = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap();

        let local = tokio::task::LocalSet::new();
        local.block_on(&runtime, async move {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            addr_sender.send(listener.local_addr().unwrap()).unwrap();

            let (stream, _) = listener.accept().await.unwrap();
            stream.set_nodelay(true).unwrap();
            let (reader, writer) =
                tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
            let mut network = twoparty::VatNetwork::new(
                futures::io::BufReader::new(reader),
                futures::io::BufWriter::new(writer),
                rpc_twoparty_capnp::Side::Server,
                Default::default(),
            );
            network.set_window_size(WINDOW_SIZE);

            let sender: sender::Client = capnp_rpc::new_client(SenderImpl);
            RpcSystem::new(Box::new(network), Some(sender.client))
                .await
                .unwrap();
        });
    });

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .unwrap();
    let local = tokio::task::LocalSet::new();
    local.block_on(&runtime, async move {
        let stream = tokio::net::TcpStream::connect(addr_receiver.recv().unwrap())
            .await
            .unwrap();
        stream.set_nodelay(true).unwrap();
        let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
        let mut network = twoparty::VatNetwork::new(
            futures::io::BufReader::new(reader),
            futures::io::BufWriter::new(writer),
            rpc_twoparty_capnp::Side::Client,
            Default::default(),
        );
        network.set_window_size(WINDOW_SIZE);

        let mut rpc_system = RpcSystem::new(Box::new(network), None);
        let sender: sender::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
        let disconnector = rpc_system.get_disconnector();
        tokio::task::spawn_local(rpc_system);

        let (done_sender, done_receiver) = oneshot::channel();
        let stream: byte_stream::Client = capnp_rpc::new_client(ByteStreamImpl::new(done_sender));
        let mut request = sender.send_request();
        request.get().set_stream(stream);
        request.get().set_size(STREAM_SIZE);
        request.get().set_chunk_size(CHUNK_SIZE);

        let local_result = async move {
            done_receiver
                .await
                .map_err(|_| Error::failed("ByteStream.end() was not called".into()))
        };
        let wait_promise = request
            .send()
            .pipeline
            .get_transfer()
            .wait_request()
            .send()
            .promise;
        let (response, (local_sha256, bytes_received)) =
            futures::future::try_join(wait_promise, local_result)
                .await
                .unwrap();

        assert_eq!(bytes_received, STREAM_SIZE);
        assert_eq!(
            response.get().unwrap().get_sha256().unwrap(),
            &local_sha256[..]
        );

        disconnector.await.unwrap();
    });

    server_thread.join().unwrap();
}
