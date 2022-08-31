// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#[cfg(any(windows, unix))]
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Barrier,
    },
    thread,
    time::Duration,
};

pub trait RwLock<T> {
    fn new(v: T) -> Self;

    fn read<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&T) -> R;

    fn write<F, R>(&self, f: F) -> R
        where
            F: FnOnce(&mut T) -> R;

    fn name() -> &'static str;
}

fn run_benchmark<M: RwLock<f64> + Send + Sync + 'static>(
    num_writer_threads: usize,
    num_reader_threads: usize,
    work_per_critical_section: usize,
    work_between_critical_sections: usize,
    seconds_per_test: f32,
) -> (Vec<usize>, Vec<usize>) {
    let lock = Arc::new(([0u8; 300], M::new(0.0), [0u8; 300]));
    let keep_going = Arc::new(AtomicBool::new(true));
    let barrier = Arc::new(Barrier::new(num_reader_threads + num_writer_threads));
    let mut writers = vec![];
    let mut readers = vec![];
    for _ in 0..num_writer_threads {
        let barrier = barrier.clone();
        let lock = lock.clone();
        let keep_going = keep_going.clone();
        writers.push(thread::spawn(move || {
            let mut local_value = 0.0;
            let mut value = 0.0;
            let mut iterations = 0usize;
            barrier.wait();
            while keep_going.load(Ordering::Relaxed) {
                lock.1.write(|shared_value| {
                    for _ in 0..work_per_critical_section {
                        *shared_value += value;
                        *shared_value *= 1.01;
                        value = *shared_value;
                    }
                });
                for _ in 0..work_between_critical_sections {
                    local_value += value;
                    local_value *= 1.01;
                    value = local_value;
                }
                iterations += 1;
            }
            (iterations, value)
        }));
    }
    for _ in 0..num_reader_threads {
        let barrier = barrier.clone();
        let lock = lock.clone();
        let keep_going = keep_going.clone();
        readers.push(thread::spawn(move || {
            let mut local_value = 0.0;
            let mut value = 0.0;
            let mut iterations = 0usize;
            barrier.wait();
            while keep_going.load(Ordering::Relaxed) {
                lock.1.read(|shared_value| {
                    for _ in 0..work_per_critical_section {
                        local_value += value;
                        local_value *= *shared_value;
                        value = local_value;
                    }
                });
                for _ in 0..work_between_critical_sections {
                    local_value += value;
                    local_value *= 1.01;
                    value = local_value;
                }
                iterations += 1;
            }
            (iterations, value)
        }));
    }

    thread::sleep(Duration::from_secs_f32(seconds_per_test));
    keep_going.store(false, Ordering::Relaxed);

    let run_writers = writers
        .into_iter()
        .map(|x| x.join().unwrap().0)
        .collect::<Vec<usize>>();
    let run_readers = readers
        .into_iter()
        .map(|x| x.join().unwrap().0)
        .collect::<Vec<usize>>();

    (run_writers, run_readers)
}

pub fn run_benchmark_iterations<M: RwLock<f64> + Send + Sync + 'static>(
    num_writer_threads: usize,
    num_reader_threads: usize,
    work_per_critical_section: usize,
    work_between_critical_sections: usize,
    seconds_per_test: f32,
    test_iterations: usize,
) {
    let mut writers = vec![];
    let mut readers = vec![];

    for _ in 0..test_iterations {
        let (run_writers, run_readers) = run_benchmark::<M>(
            num_writer_threads,
            num_reader_threads,
            work_per_critical_section,
            work_between_critical_sections,
            seconds_per_test,
        );
        writers.extend_from_slice(&run_writers);
        readers.extend_from_slice(&run_readers);
    }

    let total_writers = writers.iter().fold(0f64, |a, b| a + *b as f64) / test_iterations as f64;
    let total_readers = readers.iter().fold(0f64, |a, b| a + *b as f64) / test_iterations as f64;
    println!(
        "{:46} - [write] {:10.3} kHz          [read] {:10.3} kHz",
        M::name(),
        total_writers as f64 / seconds_per_test as f64 / 1000.0,
        total_readers as f64 / seconds_per_test as f64 / 1000.0
    );
}