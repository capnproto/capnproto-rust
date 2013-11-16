#! /bin/bash

set -x

time ./benchmark carsales object no-reuse none 10000
time ./benchmark carsales bytes no-reuse none 10000
time ./benchmark carsales bytes no-reuse packed 10000
time ./benchmark carsales pipe no-reuse none 10000
time ./benchmark carsales pipe no-reuse packed 10000

time ./benchmark catrank object no-reuse none 1000
time ./benchmark catrank bytes no-reuse none 1000
time ./benchmark catrank bytes no-reuse packed 1000
time ./benchmark catrank pipe no-reuse none 1000
time ./benchmark catrank pipe no-reuse packed 1000

time ./benchmark eval object no-reuse none 200000
time ./benchmark eval bytes no-reuse none 200000
time ./benchmark eval bytes no-reuse packed 200000
time ./benchmark eval pipe no-reuse none 200000
time ./benchmark eval pipe no-reuse packed 200000

