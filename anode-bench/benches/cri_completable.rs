use std::time::Duration;
use anode::completable::Completable;
use criterion::{criterion_group, criterion_main, Criterion, black_box};

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

    let completable = Completable::new(());
    c.bench_function("completed/complete", |b| {
        b.iter(|| {
            let completed = completable.complete(());
            debug_assert!(completed.is_some());
            completed
        });
    });

    let completable = Completable::new(());
    c.bench_function("completed/get", |b| {
        b.iter(|| {
            let completed = completable.get();
            black_box(completed);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);