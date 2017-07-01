---
layout: post
title: updated benchmark comparison to C++
author: dwrensha
---

Two months have passed since I posted an
[initial benchmark]({{site.baseurl}}/2013/11/16/benchmark.html)
of the Rust and C++ implementations of [Cap'n Proto](http://kentonv.github.io/capnproto/).
Enough has changed in that time to make it worth
presenting updated results.

One change is that I got a faster computer. Unfortunately,
that means that results from the two benchmarks will not be directly
comparable.

A more pertinent change
is that I (partially) implemented scratch-space reuse,
an optional feature that can reduce allocations.
The addition of this feature doubles the number of benchmark configurations,
as we can run each communication mode with or without "reuse" turned on.

Otherwise, the benchmark itself is the same as before.

First, the "carsales" case, heavy on numbers.

<img src="{{site.baseurl}}/assets/carsales-2014-01-14.png"
     width="500"/>

Recall that in November's benchmark,
capnproto-rust was slightly faster than capnproto-c++
in "object" mode.
This is no longer true.
I believe that capnproto-c++
was previously disadvantaged
because it was providing
extra thread safety&mdash;in particular,
the ability for multiple threads
to share a mutable MessageBuilder.
That feature was dropped in
[this commit](https://github.com/kentonv/capnproto/commit/c5bed0d2967193b095f980341fd88dc7decd5e94).

Next, the "catrank" case, heavy on strings.

<img src="{{site.baseurl}}/assets/catrank-2014-01-14.png"
     width="500"/>

In November, capnproto-rust was hampered here
by its lack of support for
direct writing of string fields.
That has been remedied.
However, capnproto-rust
has another disadvantage here;
cpu profiling reveals that
it spends roughly ten percent of its time verifying that strings
are valid UTF-8, while capnproto-c++
does not bother with any such verification.
Note that the Cap'n Proto [encoding spec](http://kentonv.github.io/capnproto/encoding.html#blobs)
requires that strings be valid UTF-8, but says
nothing about whether
the receiver of a non-UTF-8 string
should report an error.

Finally, the "eval" case, heavy on pointer indirections.
<img src="{{site.baseurl}}/assets/eval-2014-01-14.png"
     width="500"/>


In contrast to November's results,
the relative performance of capnproto-rust now does not
significantly degrade when it must perform I/O in the "pipe"
communication mode.
The main reason for the improvement is
that the benchmark now uses libnative, Rust's 1:1 threading runtime,
whereas in November
it used libgreen,
Rust's M:N threading runtime built on libuv.
Only recently has it become convenient to swap between
these two runtimes.
If I run the "eval" case in "pipe" mode with libgreen today,
Rust takes approximately twice as long as it does with libnative.

