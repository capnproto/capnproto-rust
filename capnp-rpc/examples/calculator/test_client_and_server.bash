 #! /bin/bash

set -x
set -e

cargo build --package calculator
../../../target/debug/calculator server 127.0.0.1:6569 &
SERVER=$!
sleep 1
../../../target/debug/calculator client 127.0.0.1:6569

kill $SERVER

