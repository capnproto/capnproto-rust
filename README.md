# capnproto-rust: Cap'n Proto for Rust

## About

[Cap'n Proto](http://kentonv.github.io/capnproto/) is a
data interchange format designed for extreme efficiency.

capnproto-rust is a (work-in-progress) implementation of Cap'n Proto
for [Rust](http://www.rust-lang.org).

## Getting Started

To compile, get the latest (master branch) version of Rust and do:
```
$ rustc capnprust/capnprust.rs
```
This should succeed and produce
the library `libcapnprust`. You may then compile the `capnpc-rust` binary like this:
```
$ rustc -L./capnprust compiler/capnpc-rust.rs
```
The binary may then be used as a plug-in to
the Cap'n Proto compiler, like this:

```
$ capnpc -o ./compiler/capnpc-rust:samples samples/addressbook.capnp
```
This should generate the file `samples/addressbook_capnp.rs`.
You may then see the serialization in action by compiling the sample program:

```
$ rustc -L./capnprust samples/addressbook.rs
$ ./samples/addressbook write | ./samples/addressbook read
```

## Implementation Notes

The general strategy is to translate, as directly as possible, the C++
implementation into Rust. (Comments that have been directly copied
from the C++ implementation are demarked with a double slash and pound
sign `//#`.) Fortunately, enums and structs are laid out the same way
in both languages. Unfortunately, trait polymorphism and region
variables do not seem to work very well together yet in Rust. This
makes a few things difficult, including implementing virtual functions
such as `MessageReader::getSegment()`.

## Status

capnproto-rust is a work in progress, and the parts that are done are
not well tested. The core features are more or less operational,
including reading and writing of messages, packed serialization, and
Rust code generation. There are several prominent missing features,
including non-text blobs and nested lists.

The next major implementation task will probably be to set up some
benchmarks.




