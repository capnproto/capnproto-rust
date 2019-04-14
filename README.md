# Cap'n Proto for Rust

[![Build Status](https://travis-ci.org/capnproto/capnproto-rust.svg?branch=master)](https://travis-ci.org/capnproto/capnproto-rust)

[documentation](https://docs.capnproto-rust.org/capnp/)

[blog](https://dwrensha.github.io/capnproto-rust)

## Introduction

[Cap'n Proto](https://capnproto.org) is a type system for distributed systems.

With Cap'n Proto, you describe your data and interfaces
in a [schema file](https://capnproto.org/language.html), like this:

```capnp
@0x986b3393db1396c9;

struct Point {
    x @0 :Float32;
    y @1 :Float32;
}

interface PointTracker {
    addPoint @0 (p :Point) -> (totalPoints :UInt64);
}
```

You can then use the [capnp tool](https://capnproto.org/capnp-tool.html#compiling-schemas)
to generate code in a [variety of programming languages](https://capnproto.org/otherlang.html).
The generated code lets you produce and consume values of the
types you've defined in your schema.

Values are encoded in [a format](https://capnproto.org/encoding.html) that
is suitable not only for transmission over a network and persistence to disk,
but also for zero-copy in-memory traversal.
That is, you can completely skip serialization and deserialization!
It's in this sense that Cap'n Proto is
["infinity times faster"](https://capnproto.org/news/2013-04-01-announcing-capn-proto.html)
than alternatives like Protocol Buffers.

In Rust, the generated code for the example above includes
a `point::Reader<'a>` struct with `get_x()` and `get_y()` methods,
and a `point::Builder<'a>` struct with `set_x()` and `set_y()` methods.
The lifetime parameter `'a` is a formal reminder
that `point::Reader<'a>` and `point::Builder<'a>`
contain borrowed references to the raw buffers that contain the encoded messages.
Those underlying buffers are never actually copied into separate data structures.

The generated code for the example above also includes
a `point_tracker::Server` trait with an `add_point()` method,
and a `point_tracker::Client` struct with an `add_point_request()` method.
The former can be implemented to create a network-accessible object,
and the latter can be used to invoke a possibly-remote instance of a `PointTracker`.

## Features

- [tagged unions](https://capnproto.org/language.html#unions)
- [generics](https://capnproto.org/language.html#generic-types)
- [protocol evolvability](https://capnproto.org/language.html#evolving-your-protocol)
- [canonicalization](https://capnproto.org/encoding.html#canonicalization)
- [`Result`-based error handling](https://dwrensha.github.io/capnproto-rust/2015/03/21/error-handling-revisited.html)

## Crates

|  |  |  |
| ----- | ---- | ---- |
| [capnp](/capnp) | Runtime library for dealing with Cap'n Proto messages. | [![crates.io](http://meritbadge.herokuapp.com/capnp)](https://crates.io/crates/capnp) |
| [capnpc](/capnpc) | Rust code generator [plugin](https://capnproto.org/otherlang.html#how-to-write-compiler-plugins), including support for hooking into a `build.rs` file in a `cargo` build. | [![crates.io](http://meritbadge.herokuapp.com/capnpc)](https://crates.io/crates/capnpc) |
| [capnp-futures](/capnp-futures) | Support for asynchronous reading and writing of Cap'n Proto messages. | [![crates.io](http://meritbadge.herokuapp.com/capnp-futures)](https://crates.io/crates/capnp-futures) |
| [capnp-rpc](/capnp-rpc) | Object-capability remote procedure call system. | [![crates.io](http://meritbadge.herokuapp.com/capnp-rpc)](https://crates.io/crates/capnp-rpc) |

## Examples

[addressbook serialization](/example/addressbook),
[RPC](/capnp-rpc/examples)

## Who is using capnproto-rust?

- Sandstorm's [raw API example app](https://github.com/dwrensha/sandstorm-rawapi-example-rust) and
  [collections app](https://github.com/sandstorm-io/collections-app)
- [leaf](https://github.com/autumnai/leaf)
- [fractalide](https://github.com/fractalide/fractalide)
- [combustion-engine](https://github.com/combustion-engine/combustion/tree/master/combustion_protocols)

## Unimplemented / Future Work

- [orphans](https://capnproto.org/cxx.html#orphans)
- [dynamic reflection](https://capnproto.org/cxx.html#dynamic-reflection)
