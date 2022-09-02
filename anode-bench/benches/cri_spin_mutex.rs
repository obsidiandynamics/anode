use criterion::{criterion_group, criterion_main, Criterion};
use anode::spin_mutex::SpinMutex;

fn criterion_benchmark(c: &mut Criterion) {
    let mutex = SpinMutex::new(());
    c.bench_function("lock", |b| {
        b.iter(|| mutex.lock());
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
