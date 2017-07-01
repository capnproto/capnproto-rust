---
layout: post
title: gj I/O split into its own crate
author: dwrensha
---

Version 0.2.0 of the [Good Job Event Loop](https://github.com/dwrensha/gj)
is now [live on crates.io](https://crates.io/crates/gj).
The major change in this release is that all of the
I/O code has been split out into a separate crate
called
[gjio](https://github.com/dwrensha/gjio).
The main gj crate still defines the core `Promise`
and `EventLoop` structures,
but gjio implements all the specifics about how events are
received from the outside world.
The [`EventPort`](https://docs.rs/gjio/0.1.3/gjio/struct.EventPort.html) trait
is the hook that allows gj to use those specifics.
If for whatever reason you decide you don't like gjio, you can write your own
`EventPort` impelemention and still use gj.

As I've moved the `gj::io` module into its own crate,
I've taken the opportunity to iterate somewhat on the design.
Probably the most prominent change is
that the methods of [`AsyncRead`](https://docs.rs/gjio/0.1.3/gjio/trait.AsyncRead.html)
and [`AsyncWrite`](https://docs.rs/gjio/0.1.3/gjio/trait.AsyncWrite.html)
now take `self` by reference
rather than by move, which I think is an ergnomonic win.
Judge for yourself by checking out the
[old version](https://github.com/dwrensha/gj/blob/v0.1.2/examples/echo.rs)
and the
[new version](https://github.com/dwrensha/gjio/blob/v0.1.0/examples/echo.rs)
of a TCP echo example.

A notable under-the-hood change is that, unlike the old `gj:io` module,
the new gjio crate implements its own custom low level
[system](https://github.com/dwrensha/gjio/blob/v0.1.0/src/sys/unix/epoll.rs)
[specific](https://github.com/dwrensha/gjio/blob/v0.1.0/src/sys/unix/kqueue.rs)
[code](https://github.com/dwrensha/gjio/blob/v0.1.0/src/sys/windows/mod.rs)
for calling the non-blocking I/O interfaces of Linux, OSX, and Windows.
Doing so requires less code than you might think,
especially because native nonblocking I/O model on Windows
is a good match for the completion-based interfaces of gjio's
`AsyncRead` and `AsyncWrite`.






