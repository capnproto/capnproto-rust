#! /bin/sh

set -e
set -x

cargo build -p capnpc
capnp compile -otarget/debug/capnpc-rust-bootstrap:capnp/src capnp/schema.capnp --src-prefix capnp/ -I. --no-standard-import
rustfmt capnp/src/schema_capnp.rs
