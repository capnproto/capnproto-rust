#! /bin/bash

set -x

time ./benchmark carsales object reuse none 10000
time ./benchmark carsales object no-reuse none 10000
time ./benchmark carsales bytes reuse none 10000
time ./benchmark carsales bytes no-reuse none 10000
time ./benchmark carsales bytes reuse packed 10000
time ./benchmark carsales bytes no-reuse packed 10000
time ./benchmark carsales pipe reuse none 10000
time ./benchmark carsales pipe no-reuse none 10000
time ./benchmark carsales pipe reuse packed 10000
time ./benchmark carsales pipe no-reuse packed 10000

time ./benchmark catrank object reuse none 1000
time ./benchmark catrank object no-reuse none 1000
time ./benchmark catrank bytes reuse none 1000
time ./benchmark catrank bytes no-reuse none 1000
time ./benchmark catrank bytes reuse packed 1000
time ./benchmark catrank bytes no-reuse packed 1000
time ./benchmark catrank pipe reuse none 1000
time ./benchmark catrank pipe no-reuse none 1000
time ./benchmark catrank pipe reuse packed 1000
time ./benchmark catrank pipe no-reuse packed 1000

time ./benchmark eval object reuse none 200000
time ./benchmark eval object no-reuse none 200000
time ./benchmark eval bytes reuse none 200000
time ./benchmark eval bytes no-reuse none 200000
time ./benchmark eval bytes reuse packed 200000
time ./benchmark eval bytes no-reuse packed 200000
time ./benchmark eval pipe reuse none 200000
time ./benchmark eval pipe no-reuse none 200000
time ./benchmark eval pipe reuse packed 200000
time ./benchmark eval pipe no-reuse packed 200000

