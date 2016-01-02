 #! /bin/bash

set -x
set -e

cargo build
./target/debug/calculator server 127.0.0.1:6569 &
SERVER=$!
./target/debug/calculator client 127.0.0.1:6569

kill $SERVER

