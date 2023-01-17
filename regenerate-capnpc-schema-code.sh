#! /bin/sh

set -e
set -x

cargo build -p capnpc
capnp compile -otarget/debug/capnpc-rust:capnpc/src capnpc/schema.capnp --src-prefix capnpc/
