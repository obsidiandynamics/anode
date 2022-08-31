use libmutex::executor::{Executor};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc};
use std::thread;
use std::time::{Duration, Instant};
use libmutex::spinlock::SpinLock;
use libmutex::wait;
use libmutex::wait::Wait;
use crate::rate::Elapsed;

pub mod print;

#[derive(Debug, Clone)]
pub struct Options {
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct ExtendedOptions {
    pub time_check_interval: u32,
    pub debug_exits: bool,
}

impl Default for ExtendedOptions {
    fn default() -> Self {
        Self {
            time_check_interval: 100,
            debug_exits: false,
        }
    }
}

#[derive(Debug)]
pub struct BenchmarkResult {
    pub iterations: u64,
    pub elapsed: Duration,
}

impl Elapsed for BenchmarkResult {
    fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

pub fn run<E: Executor + Send + 'static>(executor: E, opts: &Options, ext_opts: &ExtendedOptions) -> BenchmarkResult {
    let time_check_interval = ext_opts.time_check_interval as u64;
    let debug_exits = ext_opts.debug_exits;
    let duration = opts.duration;

    let running = Arc::new(AtomicBool::new(true));
    let completed_tasks = Arc::new(AtomicU64::default());
    let executor = Arc::new(SpinLock::new(executor)); //TODO remove mutex later

    let start_time = Instant::now();
    let load_thread = {
        let running = running.clone();
        let completed_tasks = completed_tasks.clone();
        let executor = executor.clone();
        thread::spawn(move || {
            let mut iterations = 0u64;
            while iterations % time_check_interval != 0 || running.load(Ordering::Relaxed) {
                let completed_tasks = completed_tasks.clone();
                let executor = executor.lock();
                executor.submit(move || {
                    completed_tasks.fetch_add(1, Ordering::Relaxed);
                });
                drop(executor);
                iterations += 1;
            }

            if debug_exits {
                println!("load thread exited, expect {iterations} iterations");
            }
            iterations
        })
    };

    {
        let running = running.clone();
        thread::spawn(move || {
            thread::sleep(duration);
            if debug_exits {
                println!("terminating threads");
            }
            running.store(false, Ordering::Relaxed);
        })
        .join()
        .unwrap();
    }
    let iterations = load_thread.join().unwrap();

    wait::Spin::wait_for(move || {
        completed_tasks.load(Ordering::Relaxed) == iterations
    }, Duration::MAX).unwrap();

    BenchmarkResult {
        iterations,
        elapsed: Instant::now() - start_time,
    }
}
