#! /bin/sh

set -e
set -x

PUBLISH_DIR=~/docs.capnproto-rust.org

cargo doc --no-deps -p capnp
cargo doc --no-deps -p capnpc
cargo doc --no-deps -p capnp-rpc
rm -rf $PUBLISH_DIR/*
cp -r target/doc/* $PUBLISH_DIR/

