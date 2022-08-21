#!/bin/sh
set -e

# continuously test the system, using alternating values for the release profile and the number of test threads
while [ true ]; do
  echo "Release + single-threaded"
  cargo test --release -- --test-threads 1 micro_bench_int
  echo "Debug + single-threaded"
  cargo test -- --test-threads 1 micro_bench_int
  echo "Release + multi-threaded"
  cargo test --release micro_bench_int
  echo "Debug + multi-threaded"
  cargo test micro_bench_int
done