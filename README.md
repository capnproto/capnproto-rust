# capnproto-rust: Cap'n Proto for Rust

## About

[Cap'n Proto](http://kentonv.github.io/capnproto/) is a
data interchange format designed for extreme efficiency.

capnproto-rust is a (work-in-progress) implementation of Cap'n Proto
for [Rust](http://www.rust-lang.org).

## Getting Started

You will need an up-to-date version of Rust from the master branch.

To build capnproto-rust, just type `make` in this directory. This
should produce the library `libcapnprust`, the compiler plugin
`capnpc-rust`, and the sample program `addressbook`. You can run the
sample program like this:

```
$ ./samples/addressbook write | ./samples/addressbook read
```

## Implementation Notes

The general strategy is to translate, as directly as possible, the C++
implementation into Rust. (Comments that have been directly copied
from the C++ implementation are demarked with a double slash and pound
sign `//#`.)

## Status

capnproto-rust is a work in progress, and the parts that are done are
not well tested. The core features are more or less operational,
including reading and writing of messages, packed serialization, and
Rust code generation. There are several prominent missing features,
including non-text blobs and nested lists.

The next major implementation task will probably be to set up some
benchmarks.




