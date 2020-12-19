---
layout: post
title: capnp-rpc-rust now uses futures-rs
author: dwrensha
---

The concurrency story of
[capnp-rpc-rust](https://crates.io/crates/capnp-rpc)
gets a major update in today's version 0.8 release.
Previously, the remote procedure call system was built
on top of [GJ](https://github.com/dwrensha/gj),
an event loop framework designed specifically for Cap'n Proto,
described in some of my [previous]({{site.baseurl}}/2015/05/25/asynchronous-io-with-promises.html)
[posts]({{site.baseurl}}/2016/01/11/async-rpc.html).
The new version drops GJ in favor of
[futures-rs](https://github.com/alexcrichton/futures-rs),
a library that is quickly becoming the standard
foundation for asynchronous programming in Rust.

At the level of types, the update is fairly
straightforward.
The main asynchronous building block in GJ is the struct
`Promise<T, E>`, representing a `Result<T, E>` that might not
be ready yet. To migrate to futures-rs, each `gj::Promise<T,E>` can be translated into
a `Box<futures::Future<Item=T,Error=E>>`,
and the high-level structure of a program usually does not need to change.

Many nice properties derive from the fact that `Future` is a *trait*, not a struct,
and does not need to be put in a `Box`.
Concrete types implementing `Future` can be used in generics,
making it possible for combinators like `.then()` and `.join()`
to avoid heap allocations
and to avoid losing type information.
In particular, the typechecker can know at compile time
whether it is safe to send a future between threads!

The Rust community has a growing ecosystem of libraries based on
futures-rs, and today's capnp-rpc-rust release
should work well with all of them.
For example, a Cap'n Proto method could invoke
[futures-cpupool](https://crates.io/crates/futures-cpupool)
to distribute computation-heavy work among a pool of worker threads,
or it could use one of the emerging asynchronous database drivers
to make queries on a remote database, or it could do,
well, anything that can be expressed in terms of the `Future` trait.
As a quick demonstration, I have implemented a
simple [example](https://github.com/capnproto/capnproto-rust/tree/capnp-v0.8.17/capnp-rpc/examples/http-requests)
that uses [tokio-curl](https://github.com/tokio-rs/tokio-curl)
to make asynchronous HTTP requests.

There are many exciting possibilities to explore.
If any of this sounds interesting to you, I encourage you to get involved!
Join me for discussion at \#sandstorm on freenode IRC or at the
[tokio gitter](https://gitter.im/tokio-rs/tokio).



