use std::cmp::Ordering;
use std::sync::{Arc, Barrier};
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use crate::monitor::{Directive, Monitor, SpeculativeMonitor};
use crate::{test_utils, wait};
use crate::test_utils::{LONG_WAIT, SHORT_WAIT};
use crate::wait::{Wait, WaitResult};

#[test]
fn return_immediately() {
    let monitor = SpeculativeMonitor::new(0);
    let mut invocations = 0;
    monitor.enter(|val| {
        assert_eq!(0, *val);
        *val = 42;
        invocations += 1;
        Directive::Return
    });
    assert_eq!(1, invocations);

    let mut invocations = 0;
    monitor.enter(|val| {
        assert_eq!(42, *val);
        invocations += 1;
        Directive::Return
    });
    assert_eq!(1, invocations);
    assert_eq!(0, monitor.num_waiting());
}

#[test]
fn wait_for_nothing() {
    let monitor = SpeculativeMonitor::new(());
    let mut invocations = 0;
    monitor.enter(|_| {
        invocations += 1;
        Directive::Wait(Duration::ZERO)
    });
    // Duration::ZERO does not actually wait, so spurious wake-ups are impossible
    assert_eq!(2, invocations);
    monitor.wait_for_num_waiting(Ordering::is_eq, 0, LONG_WAIT).unwrap();

    let mut invocations = 0;
    monitor.enter(|_| {
        invocations += 1;
        Directive::Wait(SHORT_WAIT)
    });
    // spurious wake-ups are unlikely but possible; so we check for at least 2 iterations
    assert!(invocations >=2);
    monitor.wait_for_num_waiting(Ordering::is_eq, 0, LONG_WAIT).unwrap();
}

#[test]
fn notify_nothing() {
    let monitor = SpeculativeMonitor::new(());
    let mut invocations = 0;
    monitor.enter(|_| {
        invocations += 1;
        Directive::NotifyOne
    });
    assert_eq!(1, invocations);

    let mut invocations = 0;
    monitor.enter(|_| {
        invocations += 1;
        Directive::NotifyAll
    });
    assert_eq!(1, invocations);
}

#[test]
fn wait_for_notify() {
    for _ in 0..10 {
        let monitor = Arc::new(SpeculativeMonitor::new(false));

        let t_2_waited = Arc::new(Barrier::new(2));
        let t_2 = {
            let monitor = monitor.clone();
            let t_2_waited = t_2_waited.clone();
            test_utils::spawn_blocked(move || {
                monitor.enter(|flag| {
                    match flag {
                        true => {
                            *flag = false;
                            t_2_waited.wait();
                            Directive::Return
                        },
                        false => Directive::Wait(Duration::MAX)
                    }
                })
            })
        };

        assert!(!t_2.is_finished());

        // wait until t_2 is about to (or is already in) the wait state
        monitor.wait_for_num_waiting(Ordering::is_eq, 1, LONG_WAIT).unwrap();

        // raise the flag and notify one thread (there should only be one waiting)
        monitor.enter(|flag| {
            *flag = true;
            Directive::NotifyOne
        });

        // wait for t_2 to wake from the notification
        t_2_waited.wait();
        monitor.wait_for_num_waiting(Ordering::is_eq, 0, LONG_WAIT).unwrap();

        // the flag should have been lowered by the woken thread
        monitor.enter(|flag| {
            assert!(!*flag);
            Directive::Return
        });

        t_2.join().unwrap();
    }
}

#[test]
fn wait_for_notify_twice() {
    for _ in 0..10 {
        let monitor = Arc::new(SpeculativeMonitor::new(false));

        let t_2_awoken = Arc::new(AtomicBool::new(false));
        let t_2 = {
            let monitor = monitor.clone();
            let t_2_awoken = t_2_awoken.clone();
            test_utils::spawn_blocked(move || {
                monitor.enter(|flag| {
                    match flag {
                        true => {
                            *flag = false;
                            t_2_awoken.store(true, std::sync::atomic::Ordering::Relaxed);
                            Directive::Return
                        },
                        false => Directive::Wait(Duration::MAX)
                    }
                })
            })
        };

        let t_3_awoken = Arc::new(AtomicBool::new(false));
        let t_3 = {
            let monitor = monitor.clone();
            let t_3_awoken = t_3_awoken.clone();
            test_utils::spawn_blocked(move || {
                monitor.enter(|flag| {
                    match flag {
                        true => {
                            *flag = false;
                            t_3_awoken.store(true, std::sync::atomic::Ordering::Relaxed);
                            Directive::Return
                        },
                        false => Directive::Wait(Duration::MAX)
                    }
                })
            })
        };

        assert!(!t_2.is_finished());
        assert!(!t_3.is_finished());

        // wait until t_2 and t_3 are about to (or are already in) the wait state
        monitor.wait_for_num_waiting(Ordering::is_eq, 2, LONG_WAIT).unwrap();

        // raise the flag and notify one thread (out of two)
        monitor.enter(|flag| {
            *flag = true;
            Directive::NotifyOne
        });

        // wait until one of the threads wake up
        monitor.wait_for_num_waiting(Ordering::is_eq, 1, LONG_WAIT).unwrap();
        wait::Spin::wait_for(|| {
            t_2_awoken.load(std::sync::atomic::Ordering::Relaxed) || t_3_awoken.load(std::sync::atomic::Ordering::Relaxed)
        }, LONG_WAIT).unwrap();

        // check that only one thread has awoken
        assert_ne!(t_2_awoken.load(std::sync::atomic::Ordering::Relaxed), t_3_awoken.load(std::sync::atomic::Ordering::Relaxed));

        // the flag should have been lowered by the woken thread
        monitor.enter(|flag| {
            assert!(!*flag);
            Directive::Return
        });

        // raise the flag and notify the remaining thread
        monitor.enter(|flag| {
            *flag = true;
            Directive::NotifyOne
        });

        // wait until the other thread wakes up
        monitor.wait_for_num_waiting(Ordering::is_eq, 0, LONG_WAIT).unwrap();
        wait::Spin::wait_for(|| {
            t_2_awoken.load(std::sync::atomic::Ordering::Relaxed) && t_3_awoken.load(std::sync::atomic::Ordering::Relaxed)
        }, LONG_WAIT).unwrap();

        // the flag should have been lowered by the woken thread
        monitor.enter(|flag| {
            assert!(!*flag);
            Directive::Return
        });

        t_2.join().unwrap();
        t_3.join().unwrap();
    }
}

#[test]
fn wait_for_notify_all() {
    for _ in 0..10 {
        let monitor = Arc::new(SpeculativeMonitor::new(false));

        let t_2_awoken = Arc::new(AtomicBool::new(false));
        let t_2 = {
            let monitor = monitor.clone();
            let t_2_awoken = t_2_awoken.clone();
            test_utils::spawn_blocked(move || {
                monitor.enter(|flag| {
                    match flag {
                        true => {
                            t_2_awoken.store(true, std::sync::atomic::Ordering::Relaxed);
                            Directive::Return
                        },
                        false => Directive::Wait(Duration::MAX)
                    }
                })
            })
        };

        let t_3_awoken = Arc::new(AtomicBool::new(false));
        let t_3 = {
            let monitor = monitor.clone();
            let t_3_awoken = t_3_awoken.clone();
            test_utils::spawn_blocked(move || {
                monitor.enter(|flag| {
                    match flag {
                        true => {
                            t_3_awoken.store(true, std::sync::atomic::Ordering::Relaxed);
                            Directive::Return
                        },
                        false => Directive::Wait(Duration::MAX)
                    }
                })
            })
        };

        assert!(!t_2.is_finished());
        assert!(!t_3.is_finished());

        // wait until t_2 and t_3 are about to (or are already in) the wait state
        monitor.wait_for_num_waiting(Ordering::is_eq, 2, LONG_WAIT).unwrap();

        // raise the flag and notify all threads
        monitor.enter(|flag| {
            *flag = true;
            Directive::NotifyAll
        });

        // wait until both threads wake up
        monitor.wait_for_num_waiting(Ordering::is_eq, 0, LONG_WAIT).unwrap();

        t_2.join().unwrap();
        t_3.join().unwrap();
    }
}

impl<T> SpeculativeMonitor<T> {
    fn wait_for_num_waiting(&self, cmp: impl FnMut(Ordering) -> bool, target: u32, duration: Duration) -> WaitResult {
        wait::Spin::wait_for_inequality(|| self.num_waiting(), cmp, &target, duration)
    }
}
