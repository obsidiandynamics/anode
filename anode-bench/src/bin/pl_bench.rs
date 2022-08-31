// Copyright 2016 Amanieu d'Antras
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use anode_bench::args::ArgRange;
use anode_bench::{args, pl_harness};
use anode_bench::pl_shims::{ArrivalOrderedLock, ParkingLotLock, ReadBiasedLock, StdLock, StochasticLock, WriteBiasedLock};

fn run_all(
    args: &[ArgRange],
    first: &mut bool,
    num_writer_threads: usize,
    num_reader_threads: usize,
    work_per_critical_section: usize,
    work_between_critical_sections: usize,
    seconds_per_test: f32,
    test_iterations: usize,
) {
    if num_writer_threads == 0 && num_reader_threads == 0 {
        return;
    }
    if *first || !args[0].is_single() || !args[1].is_single() {
        println!(
            "- Running with {} writer threads and {} reader threads",
            num_writer_threads, num_reader_threads
        );
    }
    if *first || !args[2].is_single() || !args[3].is_single() {
        println!(
            "- {} iterations inside lock, {} iterations outside lock",
            work_per_critical_section, work_between_critical_sections
        );
    }
    if *first || !args[4].is_single() {
        println!("- {} seconds per test", seconds_per_test);
    }
    *first = false;

    pl_harness::run_benchmark_iterations::<ReadBiasedLock<f64>>(
        num_writer_threads,
        num_reader_threads,
        work_per_critical_section,
        work_between_critical_sections,
        seconds_per_test,
        test_iterations,
    );

    pl_harness::run_benchmark_iterations::<WriteBiasedLock<f64>>(
        num_writer_threads,
        num_reader_threads,
        work_per_critical_section,
        work_between_critical_sections,
        seconds_per_test,
        test_iterations,
    );

    pl_harness::run_benchmark_iterations::<ArrivalOrderedLock<f64>>(
        num_writer_threads,
        num_reader_threads,
        work_per_critical_section,
        work_between_critical_sections,
        seconds_per_test,
        test_iterations,
    );

    pl_harness::run_benchmark_iterations::<StochasticLock<f64>>(
        num_writer_threads,
        num_reader_threads,
        work_per_critical_section,
        work_between_critical_sections,
        seconds_per_test,
        test_iterations,
    );

    pl_harness::run_benchmark_iterations::<StdLock<f64>>(
        num_writer_threads,
        num_reader_threads,
        work_per_critical_section,
        work_between_critical_sections,
        seconds_per_test,
        test_iterations,
    );

    pl_harness::run_benchmark_iterations::<ParkingLotLock<f64>>(
        num_writer_threads,
        num_reader_threads,
        work_per_critical_section,
        work_between_critical_sections,
        seconds_per_test,
        test_iterations,
    );
}

fn main() {
    let args = args::parse(&[
        "numWriterThreads",
        "numReaderThreads",
        "workPerCriticalSection",
        "workBetweenCriticalSections",
        "secondsPerTest",
        "testIterations",
    ]);
    let mut first = true;
    for num_writer_threads in args[0] {
        for num_reader_threads in args[1] {
            for work_per_critical_section in args[2] {
                for work_between_critical_sections in args[3] {
                    for seconds_per_test in args[4] {
                        for test_iterations in args[5] {
                            run_all(
                                &args,
                                &mut first,
                                num_writer_threads,
                                num_reader_threads,
                                work_per_critical_section,
                                work_between_critical_sections,
                                seconds_per_test as f32,
                                test_iterations,
                            );
                        }
                    }
                }
            }
        }
    }
}
