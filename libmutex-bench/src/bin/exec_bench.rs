use std::time::Duration;
use libmutex::executor::{Executor, Queue, ThreadPool};
use libmutex_bench::{args, exec_harness};
use libmutex_bench::exec_harness::{ExtendedOptions, Options};
use libmutex_bench::exec_harness::print::{Header, Separator};

fn main() {
    const QUEUE: Queue = Queue::Bounded(10000);
    let args = args::parse(&["workers", "duration"]);
    for workers in args[0] {
        for duration in args[1] {
            let opts = Options {
                duration: Duration::from_secs(duration as u64)
            };
            println!("{}", Separator());
            println!("{}", opts);
            println!("{}", Header());
            let executor = ThreadPool::new(workers, QUEUE);
            run(&format!("anode::executor::ThreadPool({QUEUE:?})"), executor, &opts);
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
    println!("|{:45}|{result}", name);
}

