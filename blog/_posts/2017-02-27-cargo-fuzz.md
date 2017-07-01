---
layout: post
title: cargo-fuzz findings
author: dwrensha
---

After the
[announcement](https://www.reddit.com/r/rust/comments/5va0mi/cargofuzz_an_easy_way_to_fuzz_your_crates/)
of [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) last week,
I decided to try using it to search for bugs in capnproto-rust.
For a long time I had been meaning to try out
[afl.rs](https://github.com/rust-fuzz/afl.rs),
a similar fuzz-testing tool about which I had heard lots of good things,
but its nontrivial setup costs were enough to deter me.
Fortunately, cargo-fuzz is very simple and fast to get running.
It quickly found several issues in capnproto-rust, all of which have been fixed
in the [latest release](https://crates.io/crates/capnp).

#### Panic on invalid input (fixed in [6f9bbdca](https://github.com/dwrensha/capnproto-rust/commit/6f9bbdca5f77146f6f1ff0297295c3fded3a01a6))

The original
[C++ code](https://github.com/sandstorm-io/capnproto/blob/01f6d5e4ff05fcd67e968b50120dba3fbbb38afb/c%2B%2B/src/capnp/layout.c%2B%2B#L1875)
suggests that the branches for dealing with `Far` pointers in
`total_size()` and `copy_pointer()` are unreachable,
so I had translated them into panics.
It turns out that these branches can in fact be reached
due to invalid input, rather than any bugs in the code.
In the C++ version, this means that a somewhat
misleading exception will be thrown;
I've submitted [a pull request](https://github.com/sandstorm-io/capnproto/pull/421)
to make the exception more accurate.
The problem is more serious in the Rust version,
because Rust makes a sharp distinction between panics and error `Result`s.

#### CPU amplifications (fixed in [e89f162b](https://github.com/dwrensha/capnproto-rust/commit/e89f162b3545096aec77a62157463437d6959ac5), [66def413](https://github.com/dwrensha/capnproto-rust/commit/66def4134e8b4fbc2459d77e72717e445175879c))

One way to attempt to mount a denial-of-service attack on a
consumer of Cap'n Proto messages is to carefully craft messages
that could trick the consumer into doing lots of work. For example,
if you send a cyclic structure, the consumer might go into an infinite loop
trying to read it.
To protect against such attacks, message readers in capnproto-rust
have an adjustable traversal limit, indicating how many bytes
are allowed to be read before an error is returned.
Reads of zero-sized structs should also count against
this limit, as was observed in these
[bug](https://github.com/sandstorm-io/capnproto/blob/f29bb0dafbe081960f9b508528138d5f99f83b7b/security-advisories/2015-03-02-2-all-cpu-amplification.md)
[reports](https://github.com/sandstorm-io/capnproto/blob/f29bb0dafbe081960f9b508528138d5f99f83b7b/security-advisories/2015-03-05-0-c%2B%2B-addl-cpu-amplification.md)
for capnproto-c++ in 2015.
I thought that I had completely updated capnproto-rust with
fixes for this problem, but apparently I had missed two cases. :-(

#### Panics on TODOs (fixed in [4c8d5f3](https://github.com/dwrensha/capnproto-rust/commit/4c8d5f369335dc6deef6f9d1e818da5d47e2a36d), [77dc713b](https://github.com/dwrensha/capnproto-rust/commit/77dc713b8486bf61fe657cb82f5d6cb351e76306), [10f37267](https://github.com/dwrensha/capnproto-rust/commit/10f37267e3c94d861e946f91dada61fa4dc085ee), [3521d4e2](https://github.com/dwrensha/capnproto-rust/commit/3521d4e25877d038154350d9ea5621779724ca5c))

The fuzzer managed to find some explicit panics in capnproto-rust
that were filling in holes of unimplemented functionality.
I had forgotten that such holes still existed.
After cargo-fuzz found these,
I went ahead and implemented the functionality.
At first I was too lazy to write a test case to cover the new code,
but before going to bed that night I did
set up cargo-fuzz to run on it.
By the next morning, cargo-fuzz had found a memory safety issue
in the new code! Even better, the test case it generated
was rather clever and gave me a good starting point for writing the
[test](https://github.com/dwrensha/capnpc-rust/blob/4bd89ab2fccc1386b3b608a663e4adfbb199d695/test/test.rs#L772-L820)
that I later added to the capnproto-rust test suite to cover the new functionality.


