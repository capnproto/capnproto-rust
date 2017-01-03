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
    baz @0 () -> ();
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

## Implementing an interface

The generated code includes a `Server` trait for each of your interfaces.
To create an RPC-enabled object, you must implement that trait.

```rust
struct MyBar;

impl ::foo_capnp::bar::Server for MyBar {
     fn baz(&mut self,
            params: ::foo_capnp::bar::BazParams,
            results: ::foo_capnp::bar::BazResults)
        -> Promise<(), ::capnp::Error>
     {
         // ...
     }
}
```


## Calling methods on an object



