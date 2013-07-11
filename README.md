# capnproto-rust: Cap'n Proto for Rust

## About

[Cap'n Proto](http://kentonv.github.io/capnproto/) is a
data interchange format designed for extreme efficiency.

capnproto-rust is a (work-in-progress) implementation of Cap'n Proto
for [Rust](http://www.rust-lang.org).

## Getting Started

To compile, get the latest version of Rust and do:
```
$ rustc capnprust.rc
```
This should succeed and produce
the library `libcapnprust`. You may then compile the `capnpc-rust` binary like this:
```
$ rustc -L capnpc-rust.rc
```
The binary may then be used as a plug-in to
the Cap'n Proto compiler, like this:

```
$ capnpc -o ./capnpc-rust ../capnproto/c++/src/capnp/benchmark/catrank.capnp
```

Currently, this just prints to stdout some information about the input
capnp file. The eventual goal, of course, is that it will generate Rust
files that provide readers and builders for the messages defined in
the capnp file.

## Implementation Notes

The general strategy is to translate, as directly as possible, the C++
implementation into Rust. (Comments that have been directly copied
from the C++ implementation are demarked with a double slash and pound
sign `///#`.) Fortunately, enums and structs are laid out the same way
in both languages. Unfortunately, trait polymorphism and region
variables do not work very well together yet in Rust. This makes it
difficult to implement virtual functions such as
`MessageReader::getSegment()`. Therefore, for now, capnproto-rust just
has a single `MessageReader` struct and impl.

## Status

The basic readers are implemented. Default values and read limiting
have been omitted for now. Readers for `schema.capnp` are mostly
implemented; they are needed so that capnpc-rust can read its input,
and they will serve as an example for what general generated code will
look like.

Next up is getting starting on code generation and builders.

