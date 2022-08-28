use std::any;
use std::time::Duration;
use libmutex::xlock::{ArrivalOrdered, ReadBiased, WriteBiased, XLock};
use libmutex_bench::lock_spec::LockSpec;
use libmutex_bench::quad_harness;
use libmutex_bench::quad_harness::{Addable, BoxedInt, ExtendedOptions, Options};

#[test]
fn quake_micro_bench_read_biased_int() {
    __quake_micro_bench::<i64, XLock<_, ReadBiased>>();
}

#[test]
fn quake_micro_bench_read_biased_boxed_int() {
    __quake_micro_bench::<BoxedInt, XLock<_, ReadBiased>>();
}

#[test]
fn quake_micro_bench_read_biased_string() {
    __quake_micro_bench::<String, XLock<_, ReadBiased>>();
}

#[test]
fn quake_micro_bench_write_biased_int() {
    __quake_micro_bench::<i64, XLock<_, WriteBiased>>();
}

#[test]
fn quake_micro_bench_write_biased_boxed_int() {
    __quake_micro_bench::<BoxedInt, XLock<_, WriteBiased>>();
}

#[test]
fn quake_micro_bench_write_biased_string() {
    __quake_micro_bench::<String, XLock<_, WriteBiased>>();
}

#[test]
fn quake_micro_bench_arrival_ordered_int() {
    __quake_micro_bench::<i64, XLock<_, ArrivalOrdered>>();
}

#[test]
fn quake_micro_bench_arrival_ordered_boxed_int() {
    __quake_micro_bench::<BoxedInt, XLock<_, ArrivalOrdered>>();
}

#[test]
fn quake_micro_bench_arrival_ordered_string() {
    __quake_micro_bench::<String, XLock<_, ArrivalOrdered>>();
}

fn __quake_micro_bench<T: Addable, L: for<'a> LockSpec<'a, T = T> + 'static>() {
    let opts = Options {
        readers: 4,
        writers: 4,
        downgraders: 2,
        upgraders: 2,
        duration: Duration::from_millis(100),
    };
    let result = quad_harness::run::<T, L>(&opts, &ExtendedOptions::default());
    println!("|{:<120}|{}", any::type_name::<L>(), result);
}