# capnp-rpc-rust

[![Build Status](https://travis-ci.org/dwrensha/capnp-rpc-rust.svg?branch=master)](https://travis-ci.org/dwrensha/capnp-rpc-rust)
[![crates.io](http://meritbadge.herokuapp.com/capnp-rpc)](https://crates.io/crates/capnp-rpc)

[documentation](https://docs.capnproto-rust.org/capnp_rpc/)

This is a [level one](https://capnproto.org/rpc.html#protocol-features)
implementation of the Cap'n Proto remote procedure call protocol.
It is a fairly direct translation of the original
[C++ implementation](https://github.com/sandstorm-io/capnproto).

## Defining an interface

First, make sure that the
[`capnp` executable](https://capnproto.org/capnp-tool.html)
is installed on your system,
and that you have the [`capnpc`](https://crates.io/crates/capnpc) crate
in the `build-dependencies` section of your `Cargo.toml`.
Then, in a file named `foo.capnp`, define your interface:

```capnp
@0xa7ed6c5c8a98ca40;

interface Bar {
    baz @0 (x :Int32) -> (y :Int32);
}

```

Now you can invoke the schema compiler in a
[`build.rs`](http://doc.crates.io/build-script.html) file, like this:

```rust
extern crate capnpc;
fn main() {
    ::capnpc::CompilerCommand::new().file("foo.capnp").run().unwrap();
}
```

and you can include the generated code in your project like this:

```rust
pub mod foo_capnp {
  include!(concat!(env!("OUT_DIR"), "/foo_capnp.rs"));
}
```

## Calling methods on an RPC object

For each defined interface, the generated code includes a `Client` struct
that can be used to call the interface's methods. For example, the following
code calls the `Bar.baz()` method:

```rust
fn call_bar(client: ::foo_capnp::bar::Client)
   -> Box<Future<Item=i32, Error=::capnp::Error>
{
    let mut req = client.baz_request();
    req.get().set_x(11);
    Box::new(req.send().promise.and_then(|response| {
         Ok(try!(response.get()).get_y())
    }))
}
```

A `bar::Client` is a reference to a possibly-remote `Bar` object.
The Cap'n Proto RPC runtime tracks the number of such references
that are live at any given time and automatically drops the
object when none are left.

## Implementing an interface

The generated code also includes a `Server` trait for each of your interfaces.
To create an RPC-enabled object, you must implement that trait.

```rust
struct MyBar {}

impl ::foo_capnp::bar::Server for MyBar {
     fn baz(&mut self,
            params: ::foo_capnp::bar::BazParams,
            mut results: ::foo_capnp::bar::BazResults)
        -> Promise<(), ::capnp::Error>
     {
         // `pry!` is defined in capnp_rpc. It's analogous `try!`.
         results.get().set_y(pry!(params.get()).get_x() + 1);

         Promise::ok(())
     }
}
```

Then you can convert your object into a capability client like this:

```rust
let client = ::foo_capnp::bar::ToClient::new(MyBar {})).from_server::<::capnp_rpc::Server>();
```

This new `client` can now be sent across the network.
You can use it as the bootstrap capability when you construct an `RpcSystem`,
and you can pass it in RPC method arguments and results.

## Async methods





