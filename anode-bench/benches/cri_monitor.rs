use std::sync::Arc;
use std::thread;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;
use anode::monitor::{Directive, Monitor, SpeculativeMonitor};

fn criterion_benchmark(c: &mut Criterion) {
    // benchmarks with an uncontended monitor
    let monitor = SpeculativeMonitor::new(false);
    c.bench_function("hollow/enter_return", |b| {
        b.iter(|| {
            monitor.enter(|_| {
                Directive::Return
            });
        });
    });
    c.bench_function("hollow/enter_notify_one", |b| {
        b.iter(|| {
            monitor.enter(|_| {
                Directive::NotifyOne
            });
        });
    });
    c.bench_function("hollow/enter_wait_zero", |b| {
        b.iter(|| {
            monitor.enter(|_| {
                Directive::Wait(Duration::ZERO)
            });
        });
    });
    c.bench_function("hollow/enter_wait_max", |b| {
        b.iter(|| {
            let mut wait_requested = false;
            monitor.enter(|_| {
                if wait_requested {
                    Directive::Return
                } else {
                    wait_requested = true;
                    Directive::Wait(Duration::MAX)
                }
            });
        });
    });
    c.bench_function("hollow/lock", |b| {
        b.iter(|| {
            let guard = monitor.lock();
            black_box(guard);
        });
    });

    // benchmarks with one thread waiting for the monitor
    let monitor = Arc::new(SpeculativeMonitor::new(false));
    let thread = {
        let monitor = monitor.clone();
        thread::spawn(move || {
            monitor.enter(|state| {
                if *state {
                    Directive::Return
                } else {
                    Directive::Wait(Duration::MAX)
                }
            })
        })
    };

    c.bench_function("waited/enter_return", |b| {
        b.iter(|| {
            monitor.enter(|_| {
                Directive::Return
            });
        });
    });
    c.bench_function("waited/enter_notify_one", |b| {
        b.iter(|| {
            monitor.enter(|_| {
                Directive::NotifyOne
            });
        });
    });
    c.bench_function("waited/lock", |b| {
        b.iter(|| {
            let guard = monitor.lock();
            black_box(guard);
        });
    });

    monitor.enter(|state| {
        *state = true;
        Directive::NotifyOne
    });
    thread.join().unwrap();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
