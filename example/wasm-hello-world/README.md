This is an example application that passes a capnproto message to a no_std wasm function.

To build:

```
$ cd wasm-app
$ cargo build --release --target wasm32-unknown-unknown
$ cd ..
$ cargo run
```