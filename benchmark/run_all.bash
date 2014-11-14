#! /bin/bash

set -x

CARSALES_ITERS=${1-10000}

#time ./target/release/benchmark carsales object reuse none $CARSALES_ITERS
time ./target/release/benchmark carsales object no-reuse none $CARSALES_ITERS
#time ./target/release/benchmark carsales bytes reuse none $CARSALES_ITERS
time ./target/release/benchmark carsales bytes no-reuse none $CARSALES_ITERS
#time ./target/release/benchmark carsales bytes reuse packed $CARSALES_ITERS
time ./target/release/benchmark carsales bytes no-reuse packed $CARSALES_ITERS
#time ./target/release/benchmark carsales pipe reuse none $CARSALES_ITERS
time ./target/release/benchmark carsales pipe no-reuse none $CARSALES_ITERS
#time ./target/release/benchmark carsales pipe reuse packed $CARSALES_ITERS
time ./target/release/benchmark carsales pipe no-reuse packed $CARSALES_ITERS

CATRANK_ITERS=${2-1000}

#time ./target/release/benchmark catrank object reuse none $CATRANK_ITERS
time ./target/release/benchmark catrank object no-reuse none $CATRANK_ITERS
#time ./target/release/benchmark catrank bytes reuse none $CATRANK_ITERS
time ./target/release/benchmark catrank bytes no-reuse none $CATRANK_ITERS
#time ./target/release/benchmark catrank bytes reuse packed $CATRANK_ITERS
time ./target/release/benchmark catrank bytes no-reuse packed $CATRANK_ITERS
#time ./target/release/benchmark catrank pipe reuse none $CATRANK_ITERS
time ./target/release/benchmark catrank pipe no-reuse none $CATRANK_ITERS
#time ./target/release/benchmark catrank pipe reuse packed $CATRANK_ITERS
time ./target/release/benchmark catrank pipe no-reuse packed $CATRANK_ITERS

EVAL_ITERS=${3-200000}

#time ./target/release/benchmark eval object reuse none $EVAL_ITERS
time ./target/release/benchmark eval object no-reuse none $EVAL_ITERS
#time ./target/release/benchmark eval bytes reuse none $EVAL_ITERS
time ./target/release/benchmark eval bytes no-reuse none $EVAL_ITERS
#time ./target/release/benchmark eval bytes reuse packed $EVAL_ITERS
time ./target/release/benchmark eval bytes no-reuse packed $EVAL_ITERS
#time ./target/release/benchmark eval pipe reuse none $EVAL_ITERS
time ./target/release/benchmark eval pipe no-reuse none $EVAL_ITERS
#time ./target/release/benchmark eval pipe reuse packed $EVAL_ITERS
time ./target/release/benchmark eval pipe no-reuse packed $EVAL_ITERS

