capnp-rpc-rust
==============

This is an implementation of the Cap'n Proto remote procedure call protocol.

It's a fairly literal translation of the original
[C++ implementation](https://github.com/sandstorm-io/capnproto); any good ideas that you find
here were probably first present there.

This library is dependent on
the Cap'n Proto
data encoding [runtime](https://github.com/dwrensha/capnproto-rust)
and the [code generator](https://github.com/dwrensha/capnpc-rust).