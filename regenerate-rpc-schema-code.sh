#! /bin/sh

set -e
set -x

docker build --tag capnpc-rust .
docker run --rm -v "$(pwd)":/work -w /work capnpc-rust \
  capnp compile -o/usr/local/bin/capnpc-rust:capnp-rpc/src capnp-rpc/schema/rpc.capnp capnp-rpc/schema/rpc-twoparty.capnp --src-prefix capnp-rpc/schema/ -I. --no-standard-import
rustfmt capnp-rpc/src/rpc_capnp.rs capnp-rpc/src/rpc_twoparty_capnp.rs
