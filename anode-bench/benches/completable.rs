use anode::completable::Completable;
use criterion::{criterion_group, criterion_main, Criterion, black_box};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("complete", |b| {
        b.iter(|| {
            let completable = Completable::default();
            let completed = completable.complete(());
            debug_assert!(completed.is_none());
            completed
        });

        // b.iter_batched(
        //     || Completable::new(()),
        //     |completable| {
        //         let completed = completable.complete(());
        //         completed
        //     },
        //     BatchSize::SmallInput,
        // );
    });

    c.bench_function("is_complete", |b| {
        b.iter(|| {
            let completable = Completable::<()>::default();
            let completed = completable.is_complete();
            debug_assert!(!completed);
            completed
        });
    });

    c.bench_function("already_complete", |b| {
        b.iter(|| {
            let completable = Completable::new(());
            let completed = completable.complete(());
            debug_assert!(completed.is_some());
            completed
        });
    });

    c.bench_function("completed_get", |b| {
        b.iter(|| {
            let completable = Completable::new(());
            let completed = completable.get();
            black_box(completed);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
