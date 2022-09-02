use criterion::{criterion_group, criterion_main, Criterion};
use rand::{Rng, RngCore, thread_rng};
use anode::rand::{Probability, Rand64, Xorshift};

fn criterion_benchmark(c: &mut Criterion) {
    let mut rand = Xorshift::default();
    c.bench_function("xorshift/next_u64", |b| {
        b.iter(|| rand.next_u64());
    });

    let p = Probability::new(0.5);
    c.bench_function("xorshift/gen_bool", |b| {
        b.iter(|| rand.gen_bool(p));
    });

    let mut rand = thread_rng();
    c.bench_function("rand/next_u64", |b| {
        b.iter(|| rand.next_u64());
    });

    c.bench_function("rand/gen_bool", |b| {
        b.iter(|| rand.gen_bool(0.5));
    });

    let rand = fastrand::Rng::default();
    c.bench_function("fastrand/next_u64", |b| {
        b.iter(|| rand.u64(0..u64::MAX));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
