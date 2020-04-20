---
layout: post
title: new feature to allow unaligned buffers
author: dwrensha
---

[Last week]({{site.baseurl}}/2020/01/11/unaligned-memory-access.html) I wrote about
how capnproto-rust might relax its memory alignment requirements
and what the performance cost of that might look like.
The [ensuing discussion](https://www.reddit.com/r/rust/comments/en9fmn/should_capnprotorust_force_users_to_worry_about/)
taught me that memory alignment issues can be thornier than I had thought,
and it strengthened my belief that capnproto-rust users ought be shielded
from such issues. Since then, working with the helpful feedback
of many people, I have implemented what I consider to be a satisfactory resolution to the problem.
Today I'm releasing it as part of capnproto-rust version 0.12.
The new version not only provides a safe interface for unaligned memory, but also maintains high performance
for aligned memory.

New Feature Flag
----
Cargo supports a
[feature-flags](https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section)
mechanism, whereby a crate can declare parts of its functionality to be optional, with enablement or disablement
happening at compile time.

As of version 0.12, the `capnp` crate has a new
[feature flag called `unaligned`](https://github.com/capnproto/capnproto-rust/blob/9fa83b89eebb10aba6a1181bb7e4f9a4fad916f6/capnp/Cargo.toml#L30-L32).
When `unaligned` is enabled, `capnp` makes no assumptions about the alignment of its data.
In particular, it can read a message in place from any array of bytes via
[`read_message_from_flat_slice()`](https://github.com/capnproto/capnproto-rust/blob/9fa83b89eebb10aba6a1181bb7e4f9a4fad916f6/capnp/src/serialize.rs#L57-L65).

On the flip side, when `unaligned` is *not* enabled, `capnp` requires that message segments are 8-byte aligned,
[returning an error](https://github.com/capnproto/capnproto-rust/blob/9fa83b89eebb10aba6a1181bb7e4f9a4fad916f6/capnp/src/private/arena.rs#L92-L100)
if it detects that's not the case.
The 8-byte alignment is then used whenever
`capnp` loads or stores a primitive value in a message.

With the new interface, there is no longer a need for the problematic `unsafe fn Word::bytes_to_words()`,
so that method no longer exists.


Performance
------

The downside of enabling the `unaligned` feature is that some operations require
more instructions on certain compilation targets.
To better understand the performance cost,
I ran capnproto-rust's
[benchmark suite](https://github.com/capnproto/capnproto-rust/tree/9fa83b89eebb10aba6a1181bb7e4f9a4fad916f6/benchmark)
on three different computers: my laptop (x86_64), an EC2 ARM64 instance (aarch64), and a Raspberry Pi Zero (armv6).
I compared three different capnproto-rust versions: 0.11, 0.12, and 0.12 with `unaligned`.

As expected, on all of the computers
the 0.12 version without the `unaligned` feature performed about the same version 0.11
(within measurement noise).
When I enabled the `unaligned` feature, the only computer where there
was a noticeable performance impact was the Raspberry Pi,
where the benchmarks slowed down between 10 and 20 percent.
This also was within my expectations, though I had been hoping
it would be lower. (If the performance impact had been negligible,
I would likely not have bothered to make `unaligned` an optional feature; instead
I would have made it the *only* supported mode.)


Validation
-------

Following [ralfj's suggestion](https://www.reddit.com/r/rust/comments/en9fmn/should_capnprotorust_force_users_to_worry_about/fedr67j/),
I also performed some testing with [miri](https://github.com/rust-lang/miri)
to increase my confidence that there is no lurking undefined behavior.
I added [some tests](https://github.com/capnproto/capnproto-rust/blob/9fa83b89eebb10aba6a1181bb7e4f9a4fad916f6/capnp/src/private/layout_test.rs#L24)
that specifically force 1-byte alignment.


I was pleasantly surprised to learn how easy it is to run miri these days:

```
$ rustup component add miri
$ cargo miri test
```

I recommend that you try this on your own projects!



