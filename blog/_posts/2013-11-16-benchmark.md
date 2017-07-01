---
layout: post
title: benchmark comparison to C++
author: dwrensha
---

Below I describe the results of a
performance comparison between
[capnproto-rust](https://www.github.com/dwrensha/capnproto-rust)
and the definitive
[C++ implementation](https://www.github.com/kentonv/capnproto).


To perform the comparison,
I reimplemented in Rust the three benchmark cases
included with capnproto-c++.
Each case consists of some communications
between a client and a server,
with
nontrivial computation taking place at both ends.

Within each case,
there are five possible modes of communication.
The simplest is "object", in which the sender
directly passes an in-memory object.
In "bytes", the sender writes
its message to a byte buffer, which
it then passes directly.
In "pipe", the sender writes
its message as bytes over a pipe
to another process.
In the "packed" versions of "bytes" and "pipe",
the sender applies Cap'n Proto standard packing
to the raw bytes before sending.

The "carsales" case emphasizes
records with many numeric fields.
For each communication mode,
I ran 10000 iterations,
resulting in a
throughput of about 125 MB unpacked, or 81 MB packed.

<img src="{{site.baseurl}}/assets/carsales.png"
     width="500"/>

Here capnproto-rust is roughly even with capnproto-c++
except for the cases where it has to do I/O, in which case
relative performance degrades slightly.

The "catrank" emphasizes string processing.
I ran 1000 iterations for each mode, resulting in a
throughput of about 206 MB unpacked, or 186 MB packed.

<img src="{{site.baseurl}}/assets/catrank.png"
     width="500"/>

Here capnproto-rust's relative performance is fairly consistent,
and a bit worse than in the "carsales" case. I believe
the main reason for this is that capnproto-rust is doing extra string
copies to get around the fact that I have not yet implemented direct
writing for string fields.

The "eval" case emphasizes nested structures.
I ran 200000 iterations for each mode,
resulting in a throughput of about 262 MB unpacked,
or 80 MB packed.

<img src="{{site.baseurl}}/assets/eval.png"
     width="500"/>

Here we see capnproto-rust's relative performance degrade
significantly when it must perform I/O.
I am uncertain why this degradation occurs,
though it seems to be related to the size of the messages.
Unpacked, the total size of a
single "eval" request plus a single "eval" response
is about 1 KB,
compared to the averages
of about 12 KB in the "carsales" case
and about 200 KB in the "catrank" case.

