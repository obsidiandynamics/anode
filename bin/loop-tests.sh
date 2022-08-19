#!/bin/sh

# continuously test the system, using alternating values for the release profile and the number of test threads
while [ true ]; do
  cargo test --release -- --test-threads 1
  cargo test -- --test-threads 1
  cargo test --release
  cargo test
done