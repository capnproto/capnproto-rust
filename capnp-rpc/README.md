# capnp-rpc-rust

[![crates.io](https://img.shields.io/crates/v/capnp-rpc.svg)](https://crates.io/crates/capnp-rpc)

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

interface Qux {
    quux @0 (bar :Bar) -> (y :Int32);
}
```

Now you can invoke the schema compiler in a
[`build.rs`](http://doc.crates.io/build-script.html) file, like this:

```rust
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
   -> Box<Future<Item=i32, Error=::capnp::Error>>
{
    let mut req = client.baz_request();
    req.get().set_x(11);
    Box::new(req.send().promise.and_then(|response| {
         Ok(response.get()?.get_y())
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
         // `pry!` is defined in capnp_rpc. It's analogous to `try!`.
         results.get().set_y(pry!(params.get()).get_x() + 1);

         Promise::ok(())
     }
}
```

Then you can convert your object into a capability client like this:

```rust
let client: foo_capnp::bar::Client = capnp_rpc::new_client(MyBar {});
```

This new `client` can now be sent across the network.
You can use it as the bootstrap capability when you construct an `RpcSystem`,
and you can pass it in RPC method arguments and results.

## Async methods

The methods of the generated `Server` traits return
a value of type `Promise<(), ::capnp::Error>`.
A `Promise` is either an immediate value, constructed by `Promise::ok()` or
`Promise::err()`, or it is a wrapper of a `Future`, constructed by
`Promise::from_future()`.
The results will be sent back to the method's caller once two things have happened:

  1. The `Results` struct has been dropped.
  2. The returned `Promise` has resolved.

Usually (1) happens before (2).

Here's an example of a method implementation that does not return immediately:

```rust
struct MyQux {}

impl ::foo_capnp::qux::Server for MyQux {
     fn quux(&mut self,
             params: ::foo_capnp::qux::QuuxParams,
             mut results: ::foo_capnp::wux::QuuxResults)
        -> Promise<(), ::capnp::Error>
     {
         // Call `baz()` on the passed-in client.

         let bar_client = pry!(pry!(params.get()).get_bar());
         let mut req = bar_client.baz_request();
         req.get().set_x(42);
         Promise::from_future(req.send().promise.and_then(move |response| {
             results.get().set_y(response.get()?.get_y());
             Ok(())
         }))
     }
}
```

It's possible for multiple calls of `quux()` to be active at the same time
on the same object, and they do not need to return in the same order
as they were called.

## Further reading

  * The [hello world example](/capnp-rpc/examples/hello-world) demonstrates a basic request/reply pattern.
  * The [calculator example](/capnp-rpc/examples/calculator)
    demonstrates how to use [promise pipelining](https://capnproto.org/rpc.html#time-travel-promise-pipelining).
  * The [pubsub example](/capnp-rpc/examples/pubsub) shows how even an interface with no methods can be useful.
  * The [Sandstorm raw API example app](https://github.com/dwrensha/sandstorm-rawapi-example-rust)
    shows how Sandstorm lets you write web apps using Cap'n Proto instead of HTTP.
