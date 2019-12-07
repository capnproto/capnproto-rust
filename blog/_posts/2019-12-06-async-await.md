---
layout: post
title: async/await
author: dwrensha
---

Today I'm releasing version 0.11.0 of capnproto-rust,
with support for
[async/await](https://blog.rust-lang.org/2019/11/07/Async-await-stable.html)!
The updated RPC system works with any futures-0.3-enabled executor
(e.g. tokio, async-std) -- you just need to provide it
with objects that implement the `futures::io::AsyncRead` and `futures::io::AsyncWrite` traits.

The stabilization of `std::future::Future` allowed me to eliminate
an annoying
[optional dependecy](https://github.com/capnproto/capnproto-rust/commit/0e825eecbf2337d1fb2caed015bfa4862a195d40#diff-c0b507abb73596f7f82a1c80ac680e54L31)
on futures-0.1
in the base `capnp` crate, and in general the update allowed me to
[delete a lot of code](https://github.com/capnproto/capnproto-rust/commit/0e825eecbf2337d1fb2caed015bfa4862a195d40).

In my experience, async/await can vastly simplify concurrent programming,
especially in the case where you have a single-threaded event loop
and you want to share mutable data among multiple tasks.

Probably the hardest part of this update was wrapping my head around `Pin<T>`.
My biggest takeaway message on that topic is:
if you get into trouble, try wrapping your object with `Box::pin()`.
Curiously, doing so will give you an object that is `Unpin` -- which is often
exactly what you need!


