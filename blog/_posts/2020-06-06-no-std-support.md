---
layout: post
title: no_std support
author: dwrensha
---

Over the past few years,
[many people have expressed interest](https://github.com/capnproto/capnproto-rust/issues/71)
in using capnproto-rust in [no_std](https://rust-embedded.github.io/book/intro/no-std.html) environments
-- that is, without pulling in the Rust standard library.
Today I'm happy to announce that the latest release, version 0.13.0, supports that.

To use a `no_std` capnproto-rust,
update your `Cargo.toml` to the new `capnp` version and disable default features, like this:

```toml
[capnp.dependencies]
version = "0.13"
default-features = false
```

This turns off the new
["std" feature flag](https://github.com/capnproto/capnproto-rust/blob/e2836823318d95668f10443d9f2feea8378ae95f/capnp/Cargo.toml#L36-L38)
in the `capnp` crate.
In turn, that feature controls a
[crate-level `no_std` attribute](https://github.com/capnproto/capnproto-rust/blob/e2836823318d95668f10443d9f2feea8378ae95f/capnp/src/lib.rs#L30)
and gates the parts of the crate that depend on the standard library.

## Example

To see `no_std` capnproto-rust in action,
check out this [new example](https://github.com/capnproto/capnproto-rust/tree/master/example/wasm-hello-world)
that passes data to a WebAssembly function through a Cap'n Proto message.
I observed the size of this example's generated wasm code to shrink from
1.6MB down to 660KB when I added `#![no_std]`.

## I/O traits

The biggest challenge in getting capnproto-rust to work with `no_std` was dealing with
input/output traits.
In previous releases, capnproto-rust defined its main serialization functions in terms of
`std::io::Read` and `std::io::Write`. That would be a problem in a `no_std` context,
because those traits are [stuck in `std`](https://github.com/rust-lang/rust/issues/48331).

The solution I settled on was to define custom
[`capnp::io::Read`](https://github.com/capnproto/capnproto-rust/blob/e2836823318d95668f10443d9f2feea8378ae95f/capnp/src/io.rs#L9)
and
[`capnp::io::Write`](https://github.com/capnproto/capnproto-rust/blob/e2836823318d95668f10443d9f2feea8378ae95f/capnp/src/io.rs#L44)
traits, and then to define the `capnp` serialization functions in terms of those.

Blanket impls like the following then allow existing call sites to
continue to work without being altered:

```rust
#[cfg(feature="std")]
mod std_impls {
  impl <R: std::io::Read> crate::io::Read for R {
    ...
  }
  impl <W: std::io::Write> crate::io::Write for W {
    ...
  }
}
```

## Why now?

Two recent Rust developments paved the way for today's release:

 1. The [stabilization of the alloc crate](https://github.com/rust-lang/rust/pull/59675)
    means that collections like `Vec` are now usable with `no_std`. (capnproto-rust strives
    to minimize allocations, but still relies on the global allocator for some things like
    messages with a dynamic number of segments.)
 2. [no_std support for async/await](https://github.com/rust-lang/rust/pull/69033) means that
    we can use `async` blocks wherever we want. Previously, we would have needed to define
    some custom `Future` implementations to avoid putting an `async` block in the `capnp` crate.

## Thanks

Many people contributed useful ideas in the discussion that led up to the 0.13 release.
I am especially grateful to
[nicholastmosher](https://github.com/nicholastmosher)
and [bbqsrc](https://github.com/bbqsrc)
for submitting diffs that explored the
design space.
