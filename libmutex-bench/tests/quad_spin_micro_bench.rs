use std::any;
use std::time::Duration;
use libmutex::spinlock::SpinLock;
use libmutex_bench::lock_spec::LockSpec;
use libmutex_bench::quad_harness;
use libmutex_bench::quad_harness::{Addable, BoxedInt, ExtendedOptions, Options};

#[test]
fn quad_micro_bench_int() {
    __quad_micro_bench::<i64, SpinLock<_>>();
}

#[test]
fn quad_micro_bench_boxed_int() {
    __quad_micro_bench::<BoxedInt, SpinLock<_>>();
}

#[test]
fn quad_micro_bench_string() {
    __quad_micro_bench::<String, SpinLock<_>>();
}

fn __quad_micro_bench<T: Addable, L: for<'a> LockSpec<'a, T = T> + 'static>() {
    let opts = Options {
        readers: 0,
        writers: 4,
        downgraders: 0,
        upgraders: 0,
        duration: Duration::from_millis(100),
    };
    let result = quad_harness::run::<T, L>(&opts, &ExtendedOptions::default());
    println!("|{:<120}|{}", any::type_name::<L>(), result);
}