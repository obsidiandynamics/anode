#!/bin/sh
set -e

# continuously test the system, using alternating values for the release profile and the number of test threads
while [ true ]; do
  #cargo test --release -- --test-threads 1
  #cargo test -p libmutex --lib -- --test-threads 1
  #cargo test --release
  cargo test -p libmutex --lib
done
