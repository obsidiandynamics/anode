use std::time::Duration;
use anode::executor::{Executor, Queue, ThreadPool};
use anode_bench::exec_harness;
use anode_bench::exec_harness::ExtendedOptions;
use anode_bench::exec_harness::Options;

#[test]
fn exec_micro_bench_unbounded() {
    __exec_micro_bench(ThreadPool::new(8, Queue::Unbounded));
}

#[test]
fn exec_micro_bench_bounded() {
    __exec_micro_bench(ThreadPool::new(8, Queue::Bounded(1_000)));
}

fn __exec_micro_bench<E: Executor + 'static>(executor: E) {
    let opts = Options {
        duration: Duration::from_millis(10),
    };
    let _ = exec_harness::run(executor, &opts, &ExtendedOptions::default());
}