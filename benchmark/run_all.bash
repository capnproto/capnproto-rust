#! /bin/bash

set -x

time ./benchmark carsales object no-reuse none 10000
time ./benchmark carsales bytes no-reuse none 10000
time ./benchmark carsales bytes no-reuse packed 10000
time ./benchmark carsales pipe no-reuse none 10000
time ./benchmark carsales pipe no-reuse packed 10000

