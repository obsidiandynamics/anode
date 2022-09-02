use anode::completable::Completable;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("incomplete/complete", |b| {
        b.iter(|| {
            let completable = Completable::default();
            let completed = completable.complete(());
            debug_assert!(completed.is_none());
            completed
        });
    });

    c.bench_function("incomplete/is_complete", |b| {
        b.iter(|| {
            let completable = Completable::<()>::default();
            let completed = completable.is_complete();
            debug_assert!(!completed);
            completed
        });
    });

    c.bench_function("incomplete/try_get", |b| {
        b.iter(|| {
            let completable = Completable::<()>::default();
            let maybe_completed = completable.try_get(Duration::ZERO);
            debug_assert!(maybe_completed.is_none());
            black_box(maybe_completed);
        });
    });

    let complete = Completable::new(());
    c.bench_function("complete/complete", |b| {
        b.iter(|| {
            let completed = complete.complete(());
            debug_assert!(completed.is_some());
            completed
        });
    });

    c.bench_function("complete/get", |b| {
        b.iter(|| {
            let completed = complete.get();
            black_box(completed);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
