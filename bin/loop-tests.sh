#!/bin/sh
set -e

# continuously test the system, using alternating values for the release profile and the number of test threads
while [ true ]; do
  echo "Release + single-threaded"
  cargo test --release -- --test-threads 1
  echo "Debug + single-threaded"
  cargo test -- --test-threads 1
  echo "Release + multi-threaded"
  cargo test --release
  echo "Debug + multi-threaded"
  cargo test
done