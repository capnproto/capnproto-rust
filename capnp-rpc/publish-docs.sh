#! /bin/sh

set -e
set -x

PUBLISH_DIR=/srv/http/docs.capnproto-rust.org
PUBLISH_USER=http

cargo doc --no-deps -p capnp
cargo doc --no-deps -p capnpc
cargo doc --no-deps -p capnp-rpc
cargo doc --no-deps -p capnp-futures
sudo -u $PUBLISH_USER rm -rf $PUBLISH_DIR/*
sudo -u $PUBLISH_USER cp -r target/doc/* $PUBLISH_DIR/

