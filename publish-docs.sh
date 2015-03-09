#! /bin/sh

set -e
set -x

PUBLISH_DIR=~/docs.capnproto-rust.org

cargo doc
rm -rf $PUBLISH_DIR/*
cp -r target/doc/* $PUBLISH_DIR/

