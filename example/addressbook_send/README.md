# addressbook_send example

A quick example that demonstrates how to send (parsed) Cap'N Proto messages
across thread boundaries. Because the standard `Builder` and `Reader`
interfaces require lifetimes (meaning that they're not `'static`) they can't
be sent across thread boundaries.

Make sure to have the C++ `capnp` binary and header files installed.
(For example, on Ubuntu you would install `capnproto` and `libcapnp-dev`
in your package manager.)

Try it like this:

```
$ cargo run
```
