---
layout: post
title: asynchronous I/O with promises
author: dwrensha
---


My Rust [implementation](https://github.com/dwrensha/capnp-rpc-rust)
of the Cap'n Proto remote procedure call protocol
was designed in a bygone era.
Back then, Rust's runtime library
provided thread-like "tasks"
that were backed by [libgreen](https://github.com/alexcrichton/green-rs)
and were therefore "cheap to spawn."
These enabled
[CSP](http://en.wikipedia.org/wiki/Communicating_sequential_processes)-style
programming
with beautifully simple blocking I/O operations
that were, under the hood,
dispatched through [libuv](https://github.com/libuv/libuv).
While the question of whether this model was actually efficient
was a matter of much [discussion](https://github.com/rust-lang/rfcs/pull/219),
I personally enjoyed using it and found it
easy to reason about.


For better or worse, the era of libgreen
[has ended](https://github.com/rust-lang/rfcs/blob/master/text/0230-remove-runtime.md).
Code originally written for libgreen can still work,
but because each "task" is now its own system-level thread,
calling them "lightweight" is more of a stretch than ever.
As I've maintained capnp-rpc-rust over the past year,
its need for a different approach to concurrency
has become increasingly apparent.


## Introducing [GJ](https://github.com/dwrensha/gj)

[GJ](https://github.com/dwrensha/gj) is a new Rust library that provides
abstractions for event-loop concurrency and asynchronous I/O,
aiming to meet the needs of Cap'n Proto RPC.
The main ideas in GJ are taken from
[KJ](https://capnproto.org/cxxrpc.html#kj-concurrency-framework),
a C++ library that forms the foundation of capnproto-c++.
At [Sandstorm](https://sandstorm.io), we have been
successfully using KJ-based concurrency
in our core infrastructure for a while now;
some examples you can look at include a
[bridge](https://github.com/sandstorm-io/sandstorm/blob/3a3e93eb142969125aa8573df4edc6c62efbeebe/src/sandstorm/sandstorm-http-bridge.c++) that translates between
HTTP and [this Cap'n Proto interface](https://github.com/sandstorm-io/sandstorm/blob/3a3e93eb142969125aa8573df4edc6c62efbeebe/src/sandstorm/web-session.capnp),
and a
[Cap'n Proto driver](https://github.com/sandstorm-io/sandstorm/blob/3a3e93eb142969125aa8573df4edc6c62efbeebe/src/sandstorm/fuse.c++)
to a FUSE filesystem.

The core abstraction in GJ is the `Promise<T>`, representing
a computation that may eventually resolve to a value of type `T`.
Instead of blocking, any non-immediate operation in GJ
returns a promise that gets fulfilled upon the operation's completion.
To use a promise, you register a callback with the `then()` method.
For example:

{% highlight rust %}
pub fn connect_then_write(addr: gj::io::NetworkAddress)
                         -> gj::Promise<()>
{
    return addr.connect().then(|stream| {
       // The connection has succeeded. Let's write some data.
       return Ok(stream.write(vec![1,2,3]));
    }).then(|(stream, _)| {
       // The first write has succeeded. Let's write some more.
       return Ok(stream.write(vec![4,5,6]));
    }).then(|(stream, _)| {
       // The second write has succeeded. Let's just return;
       return Ok(gj::Promise::fulfilled(()));
    });
}
{% endhighlight %}

Callbacks registered with `then()` never move between threads, so they do
not need to be thread-safe.
In Rust jargon, the callbacks are `FnOnce` closures that need not be `Send`.
This means that you can share mutable data between them
without any need for mutexes or atomics. For example, to share a counter,
you could do this:

{% highlight rust %}
pub fn ticker(counter: Rc<Cell<u32>>,
              delay_ms: u64) -> gj::Promise<()> {
    return gj::io::Timer.after_delay_ms(delay_ms).then(move |()| {
        println!("the counter is at: {}", counter.get());
        counter.set(counter.get() + 1);
        return Ok(ticker(counter, delay_ms));
    });
}

pub fn two_tickers() -> gj::Promise<Vec<()>> {
    let counter = Rc::new(Cell::new(0));
    return gj::join_promises(vec![ticker(counter.clone(), 500),
                                  ticker(counter, 750)]);
}
{% endhighlight %}


If you do want to use multiple threads, GJ makes it easy to set up an
event loop in each and to communicate between them over streams of bytes.

To learn more about what's possible with GJ,
I encourage you to explore some of more complete
[examples](https://github.com/dwrensha/gj/tree/master/examples)
in the git repo.

## Onwards!

Two things in particular have made working GJ especially fun so far:

  1. KJ is written in clean, modern C++ that translates nicely into idiomatic Rust.
     The translation is fairly direct most of the time, and parts that don't translate directly make
     for fun puzzles! For one such nontrival translation, compare KJ's
     [AsyncOutputStream](https://github.com/sandstorm-io/capnproto/blob/6315eaed384199702240c8d1b8d8186ae55e24e9/c%2B%2B/src/kj/async-io.h#L54)
     to GJ's
     [AsyncWrite](https://github.com/dwrensha/gj/blob/8156f3cc89af96024e1bc0001481b11e40bef0f5/src/io.rs#L55).
  2. The excellent [mio](https://github.com/carllerche/mio) library allows us to not worry
     about system-specific APIs. It provides a uniform abstraction on top of
     `epoll` on Linux and `kqueue` on OSX, and maybe someday even `IOCP` in Windows.

Although basics of GJ are operational today,
there's still a lot of work left to do.
If this is a project that sounds interesting
or useful to you, I'd love to have your help!


