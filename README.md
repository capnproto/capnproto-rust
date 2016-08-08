capnp-rpc-rust
==============

[![Build Status](https://travis-ci.org/dwrensha/capnp-rpc-rust.svg?branch=master)](https://travis-ci.org/dwrensha/capnp-rpc-rust)
[![crates.io](http://meritbadge.herokuapp.com/capnp-rpc)](https://crates.io/crates/capnp-rpc)

This is an implementation of the Cap'n Proto remote procedure call protocol.

It's a fairly literal translation of the original
[C++ implementation](https://github.com/sandstorm-io/capnproto); any good ideas that you find
here were probably first present there.

This library is dependent on
the Cap'n Proto
data encoding [runtime](https://github.com/dwrensha/capnproto-rust)
and the [code generator](https://github.com/dwrensha/capnpc-rust).

Documentation
-------------

See <http://docs.capnproto-rust.org/> for full documentation.
