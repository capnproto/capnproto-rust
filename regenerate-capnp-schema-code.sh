#! /bin/sh

set -e
set -x

cargo build -p capnpc
capnp compile -otarget/debug/capnpc-rust-bootstrap:capnp/src capnp/schema.capnp --src-prefix capnp/
rustfmt capnp/src/schema_capnp.rs
