#! /bin/sh

set -e
set -x

cargo build -p capnpc
capnp compile -otarget/debug/capnpc-rust-bootstrap:capnp/src/json -Icapnpc/ capnp/json.capnp --src-prefix capnp/
