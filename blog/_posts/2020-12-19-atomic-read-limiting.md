---
layout: post
title: atomic read limiting
author: dwrensha
---

Today I'm releasing capnproto-rust version 0.14.
The main change is a new
[`sync_reader`](https://github.com/capnproto/capnproto-rust/blob/c9b12bc765d5cc4e711890b97f065b855516ba71/capnp/Cargo.toml#L40-L43)
feature that allows messages to be shared between multiple threads.
With the new feature, you can, for example, wrap a `capnp::message::Reader`
with [`lazy_static`](https://crates.io/crates/lazy_static) or
[`once_cell`](https://crates.io/crates/once_cell) and then read it from anywhere else
in your program.
Previously, doing so was not possible because the
[message traversal limit](https://github.com/capnproto/capnproto-rust/blob/c9b12bc765d5cc4e711890b97f065b855516ba71/capnp/src/message.rs#L38-L55)
was tracked through a `Cell`, causing `message::Reader` to not be
[`Sync`](https://doc.rust-lang.org/std/marker/trait.Sync.html).
Now, when `sync_reader` is enabled, the traversal limit
is tracked through an `AtomicUsize`, which can be safely
shared between threads.

To minimize the performance impact, the new implementation uses
`Ordering::Relaxed` when accessing the atomic counter.
When I measured the performance on a few benchmarks,
I was initially discouraged because
[`fetch_sub()`](https://doc.rust-lang.org/std/sync/atomic/struct.AtomicUsize.html#method.fetch_sub)
seemed to be slowing things down significantly.
Fortunately, I found that splitting `fetch_sub()` into separate `load()` and `store()`
steps recovered the lost time.
(Such a split may cause the read limiter to undercount reads,
but we are okay with that level of imprecision.)
With the [most recent version](https://github.com/capnproto/capnproto-rust/blob/c9b12bc765d5cc4e711890b97f065b855516ba71/capnp/src/private/read_limiter.rs#L54-L71),
I am unable to detect any speed difference between the new atomic implementation
and the old `Cell`-based one.

I would have liked to unconditionally enable atomic read limiting,
but unfortunately `AtomicUsize` is not available on all platforms.
For example, rustc
does not support any atomics on
[riscv32i-unknown-none-elf](https://github.com/rust-lang/rust/blob/1b6b06a03a00a7c9f156bff130b72e90b79e1127/compiler/rustc_target/src/spec/riscv32i_unknown_none_elf.rs#L15).
(I am unsure whether that's an inherent property of the platform,
or whether it's an implementation hole that could be filled in later.)

[@appaquet](https://github.com/appaquet) deserves credit
for submitting [the pull request](https://github.com/capnproto/capnproto-rust/pull/201)
with this change and
for patiently iterating on it with me.