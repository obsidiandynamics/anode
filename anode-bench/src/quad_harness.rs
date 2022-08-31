use crate::lock_spec::{LockSpec, ReadGuardSpec, WriteGuardSpec};
use anode::xlock::UpgradeOutcome;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};
use std::{hint, thread};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use crate::rate::Rate;

pub mod print;

pub trait Addable: Send + Sync {
    fn initial() -> Self;

    fn get(&self) -> i64;

    fn add(&self, amount: i64) -> Self;
}

impl Addable for i64 {
    fn initial() -> Self { 0 }

    fn get(&self) -> i64 {
        *self
    }

    fn add(&self, amount: i64) -> Self {
        self + amount
    }
}

#[derive(Debug)]
pub struct BoxedInt(Box<i64>);

impl BoxedInt {
    pub fn new(v: i64) -> Self {
        Self(Box::new(v))
    }
}

impl Addable for BoxedInt {
    fn initial() -> Self {
        BoxedInt(Box::new(0))
    }

    fn get(&self) -> i64 {
        *self.0
    }

    fn add(&self, amount: i64) -> Self {
        let current = self.get();
        Self::new(current + amount)
    }
}

impl Addable for String {
    fn initial() -> Self {
        String::from("0")
    }

    fn get(&self) -> i64 {
        self.parse().unwrap()
    }

    fn add(&self, amount: i64) -> Self {
        (self.get() + amount).to_string()
    }
}

#[derive(Debug, Clone)]
pub struct Options {
    pub readers: usize,
    pub writers: usize,
    pub downgraders: usize,
    pub upgraders: usize,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct ExtendedOptions {
    pub time_check_interval: u32,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub upgrade_timeout: Duration,
    pub debug_locks: bool,
    pub debug_exits: bool,
    pub spin_inside_critical: u32,
    pub spin_outside_critical: u32,
    pub yields_inside_critical: u32,
    pub yields_outside_critical: u32,
    pub asserts_enabled: bool,
}

impl Default for ExtendedOptions {
    fn default() -> Self {
        Self {
            time_check_interval: 100,
            read_timeout: Duration::MAX,
            write_timeout: Duration::MAX,
            upgrade_timeout: Duration::ZERO,
            debug_locks: false,
            debug_exits: false,
            spin_inside_critical: 0,
            spin_outside_critical: 0,
            yields_inside_critical: 0,
            yields_outside_critical: 0,
            asserts_enabled: true,
        }
    }
}

#[derive(Debug)]
pub struct BenchmarkResult {
    pub reads: Option<u64>,
    pub writes: Option<u64>,
    pub downgrades: Option<u64>,
    pub upgrades: Option<u64>,
    pub elapsed: Duration,
}

impl BenchmarkResult {
    pub fn rate(&self, ops: u64) -> Rate {
        Rate(ops as f64 / self.elapsed.as_secs_f64())
    }

    pub fn maybe_rate(&self, ops: Option<u64>) -> Option<Rate> {
        ops.map(|ops| self.rate(ops))
    }
}

pub fn run<T: Addable, L: for<'a> LockSpec<'a, T = T> + 'static>(
    opts: &Options,
    ext_opts: &ExtendedOptions,
) -> BenchmarkResult {
    let opts = opts.clone();
    let ext_opts = ext_opts.clone();

    let readers = if L::supports_read() { opts.readers } else { 0 };
    let writers = opts.writers;
    let downgraders = if L::supports_downgrade() { opts.downgraders } else { 0 };
    let upgraders = if L::supports_upgrade() { opts.upgraders } else { 0 };
    let running = Arc::new(AtomicBool::new(true));
    let start_barrier = Arc::new(Barrier::new(
        readers + writers + downgraders + upgraders
    ));
    let lock = Arc::new(L::new(T::initial()));

    let time_check_interval = ext_opts.time_check_interval as u64;
    let reader_threads = (0..readers)
        .map(|i| {
            let running = running.clone();
            let start_barrier = start_barrier.clone();
            let lock = lock.clone();
            thread::spawn(move || {
                start_barrier.wait();
                let mut iterations = 0u64;
                let mut last_val = 0;
                while iterations % time_check_interval != 0 || running.load(Ordering::Relaxed) {
                    {
                        let val = read_eventually(&*lock, ext_opts.read_timeout);
                        if ext_opts.debug_locks {
                            println!("reader {i} read-locked");
                        }
                        spin_a_while(ext_opts.spin_inside_critical);
                        yield_a_while(ext_opts.yields_inside_critical);
                        let current = val.get();
                        if ext_opts.asserts_enabled && current < last_val {
                            panic!("error in reader {i}: value went from {last_val} to {current}");
                        }
                        last_val = current;
                    }
                    if ext_opts.debug_locks {
                        println!("reader {i} read-unlocked");
                    }
                    iterations += 1;
                    spin_a_while(ext_opts.spin_outside_critical);
                    yield_a_while(ext_opts.yields_outside_critical);
                }
                if ext_opts.debug_exits {
                    println!("reader {i} exited");
                }
                iterations
            })
        })
        .collect::<Vec<_>>();

    let writer_threads = (0..writers)
        .map(|i| {
            let running = running.clone();
            let start_barrier = start_barrier.clone();
            let lock = lock.clone();
            thread::spawn(move || {
                start_barrier.wait();
                let mut iterations = 0u64;
                while iterations % time_check_interval != 0 || running.load(Ordering::Relaxed) {
                    {
                        let mut val = write_eventually(&*lock, ext_opts.write_timeout);
                        if ext_opts.debug_locks {
                            println!("writer {i} write-locked");
                        }
                        spin_a_while(ext_opts.spin_inside_critical);
                        yield_a_while(ext_opts.yields_inside_critical);
                        *val = val.add(1);
                    }
                    if ext_opts.debug_locks {
                        println!("writer {i} write-unlocked");
                    }
                    iterations += 1;
                    spin_a_while(ext_opts.spin_outside_critical);
                    yield_a_while(ext_opts.yields_outside_critical);
                }
                if ext_opts.debug_exits {
                    println!("downgrader {i} exited");
                }
                iterations
            })
        })
        .collect::<Vec<_>>();

    let downgrader_threads = (0..downgraders)
        .map(|i| {
            let running = running.clone();
            let start_barrier = start_barrier.clone();
            let lock = lock.clone();
            thread::spawn(move || {
                start_barrier.wait();
                let mut iterations = 0u64;
                let mut last_val = 0;
                while iterations % time_check_interval != 0 || running.load(Ordering::Relaxed) {
                    {
                        let mut val = write_eventually(&*lock, ext_opts.write_timeout);
                        if ext_opts.debug_locks {
                            println!("downgrader {i} write-locked");
                        }
                        spin_a_while(ext_opts.spin_inside_critical);
                        yield_a_while(ext_opts.yields_inside_critical);
                        *val = val.add(1);

                        let val = L::downgrade(val);
                        if ext_opts.debug_locks {
                            println!("downgrader {i} downgraded");
                        }
                        let current = val.get();
                        if ext_opts.asserts_enabled && current < last_val {
                            panic!(
                                "error in downgrader {i}: value went from {last_val} to {current}"
                            );
                        }
                        last_val = current;
                    }
                    if ext_opts.debug_locks {
                        println!("downgrader {i} read-unlocked");
                    }
                    iterations += 1;
                    spin_a_while(ext_opts.spin_outside_critical);
                    yield_a_while(ext_opts.yields_outside_critical);
                }
                if ext_opts.debug_exits {
                    println!("downgrader {i} exited");
                }
                iterations
            })
        })
        .collect::<Vec<_>>();

    let upgrader_threads = (0..upgraders)
        .map(|i| {
            let running = running.clone();
            let start_barrier = start_barrier.clone();
            let lock = lock.clone();
            thread::spawn(move || {
                start_barrier.wait();
                let mut iterations = 0u64;
                let mut last_val = 0;
                let mut missed_upgrades = 0;
                while iterations % time_check_interval != 0 || running.load(Ordering::Relaxed) {
                    {
                        let val = read_eventually(&*lock, ext_opts.read_timeout);
                        if ext_opts.debug_locks {
                            println!("upgrader {i} read-locked");
                        }
                        spin_a_while(ext_opts.spin_inside_critical);
                        yield_a_while(ext_opts.yields_inside_critical);
                        let current = val.get();
                        if ext_opts.asserts_enabled && current < last_val {
                            panic!(
                                "error in upgrader {i}: value went from {last_val} to {current}"
                            );
                        }
                        last_val = current;

                        let val = L::try_upgrade(val, ext_opts.upgrade_timeout);
                        match val {
                            UpgradeOutcome::Upgraded(mut val) => {
                                if ext_opts.debug_locks {
                                    println!("upgrader {i} upgraded");
                                }
                                spin_a_while(ext_opts.spin_inside_critical);
                                yield_a_while(ext_opts.yields_inside_critical);
                                *val = val.add(1);
                                drop(val);
                                if ext_opts.debug_locks {
                                    println!("upgrader {i} write-unlocked");
                                }
                            }
                            UpgradeOutcome::Unchanged(_) => {
                                if ext_opts.debug_locks {
                                    println!("upgrader {i} upgrade timed out");
                                }
                                drop(val);
                                missed_upgrades += 1;
                                if ext_opts.debug_locks {
                                    println!("upgrader {i} read-unlocked");
                                }
                            }
                        }
                    }
                    iterations += 1;
                    spin_a_while(ext_opts.spin_outside_critical);
                    yield_a_while(ext_opts.yields_outside_critical);
                }
                if ext_opts.debug_exits {
                    println!("upgrader {i} exited");
                }
                (iterations, iterations - missed_upgrades) // (number_of_reads, number_of_upgrades)
            })
        })
        .collect::<Vec<_>>();

    let start_time = Instant::now();

    if readers + writers + upgraders + downgraders > 0 {
        let running = running.clone();
        thread::spawn(move || {
            thread::sleep(opts.duration);
            if ext_opts.debug_exits {
                println!("terminating threads");
            }
            running.store(false, Ordering::Relaxed);
        })
        .join()
        .unwrap();
    }

    let reader_iterations = reader_threads
        .into_iter()
        .map(JoinHandle::join)
        .map(Result::unwrap)
        .fold(0, |acc, reads| acc + reads);

    let writer_iterations = writer_threads
        .into_iter()
        .map(JoinHandle::join)
        .map(Result::unwrap)
        .fold(0, |acc, reads| acc + reads);

    let downgrader_iterations = downgrader_threads
        .into_iter()
        .map(JoinHandle::join)
        .map(Result::unwrap)
        .fold(0, |acc, reads| acc + reads);

    let (upgrader_reads, upgrader_upgrades) = upgrader_threads
        .into_iter()
        .map(JoinHandle::join)
        .map(Result::unwrap)
        .fold((0, 0), |(acc_reads, acc_upgrades), (reads, upgrades)| {
            (acc_reads + reads, acc_upgrades + upgrades)
        });

    BenchmarkResult {
        reads: if readers > 0 { Some(reader_iterations + upgrader_reads) } else { None },
        writes: if writers > 0 { Some(writer_iterations + downgrader_iterations) } else { None },
        downgrades: if downgraders > 0 { Some(downgrader_iterations) } else { None },
        upgrades: if upgraders > 0 { Some(upgrader_upgrades) } else { None },
        elapsed: Instant::now() - start_time,
    }
}

#[inline]
fn spin_a_while(yields: u32) {
    let mut val = 1;
    for i in 0..yields {
        hint::spin_loop();
        val += i
    }
    assert_ne!(val, 0);
}

#[inline]
fn yield_a_while(yields: u32) {
    for _ in 0..yields {
        hint::spin_loop();
        thread::yield_now();
    }
}

#[inline]
fn read_eventually<'a, T, R: ReadGuardSpec<'a, T>, L: LockSpec<'a, T = T, R = R>>(
    lock: &'a L,
    duration: Duration,
) -> R {
    let mut val = None;
    while val.is_none() {
        val = lock.try_read(duration);
    }
    val.unwrap()
}

#[inline]
fn write_eventually<'a, T, W: WriteGuardSpec<'a, T>, L: LockSpec<'a, T = T, W = W>>(
    lock: &'a L,
    duration: Duration,
) -> W {
    let mut val = None;
    while val.is_none() {
        val = lock.try_write(duration);
    }
    val.unwrap()
}
