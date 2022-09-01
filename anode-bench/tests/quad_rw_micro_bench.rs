use std::any;
use std::time::Duration;
use anode::zlock::{ArrivalOrdered, ReadBiased, Stochastic, WriteBiased, ZLock};
use anode_bench::lock_spec::LockSpec;
use anode_bench::quad_harness;
use anode_bench::quad_harness::{Addable, BoxedInt, ExtendedOptions, Options};

#[test]
fn quad_micro_bench_read_biased_int() {
    __quad_micro_bench::<i64, ZLock<_, ReadBiased>>();
}

#[test]
fn quad_micro_bench_read_biased_boxed_int() {
    __quad_micro_bench::<BoxedInt, ZLock<_, ReadBiased>>();
}

#[test]
fn quad_micro_bench_read_biased_string() {
    __quad_micro_bench::<String, ZLock<_, ReadBiased>>();
}

#[test]
fn quad_micro_bench_write_biased_int() {
    __quad_micro_bench::<i64, ZLock<_, WriteBiased>>();
}

#[test]
fn quad_micro_bench_write_biased_boxed_int() {
    __quad_micro_bench::<BoxedInt, ZLock<_, WriteBiased>>();
}

#[test]
fn quad_micro_bench_write_biased_string() {
    __quad_micro_bench::<String, ZLock<_, WriteBiased>>();
}

#[test]
fn quad_micro_bench_arrival_ordered_int() {
    __quad_micro_bench::<i64, ZLock<_, ArrivalOrdered>>();
}

#[test]
fn quad_micro_bench_arrival_ordered_boxed_int() {
    __quad_micro_bench::<BoxedInt, ZLock<_, ArrivalOrdered>>();
}

#[test]
fn quad_micro_bench_arrival_ordered_string() {
    __quad_micro_bench::<String, ZLock<_, ArrivalOrdered>>();
}

#[test]
fn quad_micro_bench_stochastic_int() {
    __quad_micro_bench::<i64, ZLock<_, Stochastic>>();
}

#[test]
fn quad_micro_bench_stochastic_boxed_int() {
    __quad_micro_bench::<BoxedInt, ZLock<_, Stochastic>>();
}

#[test]
fn quad_micro_bench_stochastic_string() {
    __quad_micro_bench::<String, ZLock<_, Stochastic>>();
}

fn __quad_micro_bench<T: Addable, L: for<'a> LockSpec<'a, T = T> + 'static>() {
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