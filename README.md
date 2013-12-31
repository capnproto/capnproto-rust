# capnproto-rust: Cap'n Proto for Rust

## About

[Cap'n Proto](http://kentonv.github.io/capnproto/) is a
data interchange format designed for extreme efficiency.

capnproto-rust is a (work-in-progress) implementation of Cap'n Proto
for [Rust](http://www.rust-lang.org).

## Getting Started

You will need Cap'n Proto and
an up-to-date version of Rust from the master branch.

To build capnproto-rust, just type `make` in this directory. This
should produce the library `libcapnp`, the compiler plugin
`capnpc-rust`, and the sample program `addressbook`. You can run the
sample program like this:

```
$ ./examples/addressbook/addressbook write | ./examples/addressbook/addressbook read
```

## Implementation Notes

The general strategy is to translate, as directly as possible, the C++
implementation into Rust. (Comments that have been directly copied
from the C++ implementation are demarked with a double slash and pound
sign `//#`.)

## Status

See updates [here](http://dwrensha.github.io/capnproto-rust).



