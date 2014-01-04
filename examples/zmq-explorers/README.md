# A toy data pipeline: zmq-explorers

This example illustrates one way to use capnroto-rust with ZeroMQ.

To build:

1. Get [libzmq](http://zeromq.org/area:download).
2. Get and build [rust-zmq](https://github.com/erickt/rust-zmq).


From the capnproto-rust root directory:

```
$ capnp compile -o./capnpc-rust/capnpc-rust examples/zmq-explorers/explorers.capnp
$ rustc examples/zmq-explorers/main.rs -L./capnp -L[rust-zmq lib path] -L[libzmq lib path]

```

Note: you may run into trouble if libzmq is installed in the same directory
as rustc [issue 11195](https://github.com/mozilla/rust/issues/11195)