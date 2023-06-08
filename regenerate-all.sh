#!/usr/bin/bash
set -euox pipefail

./regenerate-capnp-schema-code.sh
./regenerate-json-schema-code.sh
./regenerate-rpc-schema-code.sh
