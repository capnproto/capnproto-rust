use capnp_rpc::{auto_reconnect, new_future_client, rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::AsyncReadExt as _;
use tokio::net::ToSocketAddrs;

use crate::foo_capnp::foo;

pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = ::std::env::args().collect();
    if args.len() != 3 {
        println!("usage: {} client HOST:PORT", args[0]);
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
    let (foo_client, set_target) =
        auto_reconnect(move || Ok(new_future_client(connect(addr, false))))?;

    let mut request = foo_client.identity_request();
    request.get().set_x(123);
    let response = request.send().promise.await?;
    println!("results = {}", response.get()?.get_y());

    // Tell server to crash
    foo_client.crash_request().send().promise.await?;

    // Client is now disconnected and should return ErrorKind::Disconnected
    let mut request = foo_client.identity_request();
    request.get().set_x(124);
    let err = request
        .send()
        .promise
        .await
        .err()
        .ok_or("Unexpected success")?;
    if err.kind != capnp::ErrorKind::Disconnected {
        return Err(err.into());
    }
    // Retry failed request because auto_reconnect will make the connection again
    let mut request = foo_client.identity_request();
    request.get().set_x(124);
    let response = request.send().promise.await?;
    println!("results = {}", response.get()?.get_y());

    // Tell server to crash again
    foo_client.crash_request().send().promise.await?;
    // Use set_target to set new connection
    set_target.set_target(new_future_client(connect(addr, true)));

    // Send request that uses the new target
    let mut request = foo_client.identity_request();
    request.get().set_x(125);
    let response = request.send().promise.await?;
    println!("results = {}", response.get()?.get_y());

    Ok(())
}

async fn connect<A: ToSocketAddrs>(addr: A, manual: bool) -> capnp::Result<foo::Client> {
    let stream = tokio::net::TcpStream::connect(addr).await?;
    stream.set_nodelay(true)?;
    let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();

    let network = Box::new(twoparty::VatNetwork::new(
        futures::io::BufReader::new(reader),
        futures::io::BufWriter::new(writer),
        rpc_twoparty_capnp::Side::Client,
        Default::default(),
    ));

    let mut rpc_system = RpcSystem::new(network, None);
    let calculator: foo::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
    tokio::task::spawn_local(rpc_system);
    eprintln!("Connected (manual={manual})");
    Ok(calculator)
}
