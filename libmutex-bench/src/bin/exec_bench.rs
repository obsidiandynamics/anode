use std::time::Duration;
use libmutex::executor::{Executor, Queue, ThreadPool};
use libmutex_bench::{args, exec_harness};
use libmutex_bench::exec_harness::{ExtendedOptions, Options};

fn main() {
    const QUEUE: Queue = Queue::Bounded(100);
    let args = args::parse(&["workers", "duration"]);
    for workers in args[0] {
        for duration in args[1] {
            let opts = Options {
                duration: Duration::from_secs(duration as u64)
            };
            let executor = ThreadPool::new(workers, QUEUE);
            run(executor, &opts);
        }
    }
}

fn run<E: Executor + Send + 'static>(executor: E, opts: &Options) {
    let ext_opts = ExtendedOptions {
        // stick your overrides here
        ..ExtendedOptions::default()
    };
    let result = exec_harness::run(executor, opts, &ext_opts);
    println!("|{:45}|{result}", name);
}

