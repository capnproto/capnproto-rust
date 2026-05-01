#! /bin/sh

set -e
set -x

docker build --tag capnpc-rust .
docker run --rm -v "$(pwd)":/work -w /work capnpc-rust \
  capnp compile -o/usr/local/bin/capnpc-rust-bootstrap:capnp/src capnp/schema.capnp --src-prefix capnp/ -I. --no-standard-import
rustfmt capnp/src/schema_capnp.rs
