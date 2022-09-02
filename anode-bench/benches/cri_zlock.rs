use anode::zlock::{Moderator, ReadBiased, Stochastic, WriteBiased, ZLock};
use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::RwLock;

fn criterion_benchmark(c: &mut Criterion) {
    cycle(c, "read_biased", ZLock::<_, ReadBiased>::new(()));
    cycle(c, "write_biased", ZLock::<_, WriteBiased>::new(()));
    cycle(c, "stochastic", ZLock::<_, Stochastic>::new(()));

    fn cycle<M: Moderator>(c: &mut Criterion, moderator: &str, lock: ZLock<(), M>) {
        c.bench_function(&format!("{moderator}/read"), |b| {
            b.iter(|| lock.read());
        });
        c.bench_function(&format!("{moderator}/read_upgrade"), |b| {
            b.iter(|| {
                let guard = lock.read();
                guard.upgrade()
            });
        });
        c.bench_function(&format!("{moderator}/write"), |b| {
            b.iter(|| lock.write());
        });
        c.bench_function(&format!("{moderator}/write_downgrade"), |b| {
            b.iter(|| {
                let guard = lock.write();
                guard.downgrade()
            });
        });
    }

    let std_lock = RwLock::new(());
    c.bench_function("std/read", |b| {
        b.iter(|| std_lock.read());
    });
    c.bench_function("std/write", |b| {
        b.iter(|| std_lock.write());
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
