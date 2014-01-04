# zmq-explorers: a toy data pipeline

This example illustrates one way to use capnproto-rust with ZeroMQ.

## Building

Install [libzmq](http://zeromq.org/area:download) and [rust-zmq](https://github.com/erickt/rust-zmq).

From the capnproto-rust root directory:
```
$ capnp compile -o./capnpc-rust/capnpc-rust examples/zmq-explorers/explorers.capnp
$ rustc examples/zmq-explorers/main.rs -L./capnp -L[rust-zmq lib path] -L[libzmq lib path]
```
with appropriate values filled in for the library paths.
Note that you may run into trouble if libzmq is installed in the same directory
as rustc, as shown in [issue 11195](https://github.com/mozilla/rust/issues/11195).

## Running

```
$ ./examples/zmq-explorers/zmq-explorers collector
$ ./examples/zmq-explorers/zmq-explorers explorer ~/Desktop/rust_logo.ppm
$ ./examples/zmq-explorers/zmq-explorers viewer
```
