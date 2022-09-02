use criterion::{criterion_group, criterion_main, Criterion, black_box};
use anode::completable::Outcome;
use anode::executor::{Executor, Queue, Submitter, ThreadPool};

fn criterion_benchmark(c: &mut Criterion) {
    for threads in [1, 2, 4] {
        for queue_size in [100, 1_000, 10_000] {
            let pool = ThreadPool::new(threads, Queue::Bounded(queue_size));
            let submitter = pool.submitter();
            c.bench_function(&format!("submit_and_forget(threads={threads}, queue_size={queue_size})"), |b| {
                b.iter(|| {
                    let completable = submitter.submit(|| ());
                    black_box(completable);
                });
            });
        }
    }

    for threads in [1, 2, 4] {
        let pool = ThreadPool::new(threads, Queue::Unbounded);
        let submitter = pool.submitter();
        c.bench_function(&format!("submit_and_get(threads={threads})"), |b| {
            b.iter(|| {
                let completable = submitter.submit(|| ());
                let completed = completable.get();
                black_box(completed);
            });
        });
    }

    for threads in [1, 2, 4] {
        let pool = ThreadPool::new(threads, Queue::Unbounded);
        let submitter = pool.submitter();
        c.bench_function(&format!("submit_and_abort(threads={threads})"), |b| {
            b.iter(|| {
                let completable = submitter.submit(|| ());
                let maybe_aborted = completable.complete(Outcome::Abort);
                black_box(maybe_aborted);
                let completed = completable.get();
                black_box(completed);
            });
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);