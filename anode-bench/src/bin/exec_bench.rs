use std::time::Duration;
use anode::executor::{Executor, Queue, ThreadPool};
use anode_bench::{args, exec_harness};
use anode_bench::exec_harness::{ExtendedOptions, Options};
use anode_bench::exec_harness::print::{Header, Separator};

fn main() {
    let args = args::parse(&["workers", "duration"]);
    for workers in args[0] {
        for duration in args[1] {
            let opts = Options {
                duration: Duration::from_secs(duration as u64)
            };
            println!("{}", Separator());
            println!("{}", opts);
            println!("{}", Header());

            let with_queue = |queue: Queue| {
                let executor = ThreadPool::new(workers, queue.clone());
                run(&format!("anode::executor::ThreadPool(workers: {workers}, queue: {queue:?})"), executor, &opts);
            };

            with_queue(Queue::Bounded(100_000));
            with_queue(Queue::Unbounded);
        }
    }
    println!("{}", Separator());
}

fn run<E: Executor + Send + 'static>(name: &str, executor: E, opts: &Options) {
    let ext_opts = ExtendedOptions {
        // stick your overrides here
        ..ExtendedOptions::default()
    };
    let result = exec_harness::run(executor, opts, &ext_opts);
    println!("|{:70}|{result}", name);
}

