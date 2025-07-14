 #! /bin/bash

set -x
set -e

cargo build --package reconnect
../../../target/debug/reconnect server 127.0.0.1:6569 &
SERVER=$!
sleep 1
../../../target/debug/reconnect client 127.0.0.1:6569

kill $SERVER

