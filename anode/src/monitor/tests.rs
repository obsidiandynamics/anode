use std::cmp::Ordering;
use std::sync::{Arc, Barrier};
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use crate::monitor::Monitor;
use crate::monitor::Directive;
use crate::monitor::SpeculativeMonitor;
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

    let guard = monitor.lock();
    assert_eq!(42, *guard);
    drop(guard);

    let mut invocations = 0;
    monitor.enter(|val| {
        assert_eq!(42, *val);
        invocations += 1;
        Directive::Return
    });
    assert_eq!(1, invocations);
    assert_eq!(0, monitor.num_waiting());

    let guard = monitor.lock();
    assert_eq!(42, *guard);
    drop(guard);
}

#[test]
fn wait_for_nothing() {
    let monitor = SpeculativeMonitor::new(());
    let mut invocations = 0;
    let guard = monitor.enter(|_| {
        invocations += 1;
        Directive::Wait(Duration::ZERO)
    });
    assert_eq!((), *guard);
    drop(guard);
    // Duration::ZERO does not actually wait, so spurious wake-ups are impossible
    assert_eq!(1, invocations);
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
    let guard = monitor.enter(|_| {
        invocations += 1;
        Directive::NotifyOne
    });
    assert_eq!((), *guard);
    drop(guard);
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
                let guard = monitor.enter(|flag| {
                    match flag {
                        true => {
                            *flag = false;
                            t_2_waited.wait();
                            Directive::Return
                        },
                        false => Directive::Wait(Duration::MAX)
                    }
                });
                assert!(!*guard);
            })
        };

        assert!(!t_2.is_finished());

        // wait until t_2 is about to (or is already in) the wait state
        monitor.wait_for_num_waiting(Ordering::is_eq, 1, LONG_WAIT).unwrap();

        // raise the flag and notify one thread (there should only be one waiting)
        let guard = monitor.enter(|flag| {
            *flag = true;
            Directive::NotifyOne
        });
        assert!(*guard);
        drop(guard);

        // wait for t_2 to wake from the notification
        t_2_waited.wait();
        monitor.wait_for_num_waiting(Ordering::is_eq, 0, LONG_WAIT).unwrap();

        // the flag should have been lowered by the woken thread
        let guard = monitor.enter(|flag| {
            assert!(!*flag);
            Directive::Return
        });
        assert!(!*guard);
        drop(guard);

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
                });
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
                });
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
                });
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
                });
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

#[test]
fn wait_notify_chain() {
    let monitor = Arc::new(SpeculativeMonitor::new(1u8));

    let t_2_awoken = Arc::new(AtomicBool::new(false));
    let t_2 = {
        let monitor = monitor.clone();
        let t_2_awoken = t_2_awoken.clone();
        test_utils::spawn_blocked(move || {
            monitor.enter(|val| {
                match val {
                    2 => {
                        t_2_awoken.store(true, std::sync::atomic::Ordering::Relaxed);
                        *val += 1;
                        Directive::NotifyAll
                    },
                    _ => Directive::Wait(Duration::MAX)
                }
            });
        })
    };

    let t_3_awoken = Arc::new(AtomicBool::new(false));
    let t_3 = {
        let monitor = monitor.clone();
        let t_3_awoken = t_3_awoken.clone();
        test_utils::spawn_blocked(move || {
            monitor.enter(|val| {
                match val {
                    3 => {
                        t_3_awoken.store(true, std::sync::atomic::Ordering::Relaxed);
                        *val += 1;
                        Directive::NotifyOne // one remaining thread
                    },
                    _ => Directive::Wait(Duration::MAX)
                }
            });
        })
    };

    let t_4_awoken = Arc::new(AtomicBool::new(false));
    let t_4 = {
        let monitor = monitor.clone();
        let t_4_awoken = t_4_awoken.clone();
        test_utils::spawn_blocked(move || {
            monitor.enter(|val| {
                match val {
                    4 => {
                        t_4_awoken.store(true, std::sync::atomic::Ordering::Relaxed);
                        *val += 1;
                        Directive::Return // no one left to notify
                    },
                    _ => Directive::Wait(Duration::MAX)
                }
            });
        })
    };

    // initially, t_2-4 are parked
    assert!(!t_2.is_finished());
    assert!(!t_3.is_finished());
    assert!(!t_4.is_finished());

    monitor.wait_for_num_waiting(Ordering::is_eq, 3, LONG_WAIT).unwrap();

    // once the value is set to 2, t_2 should wake and cascade through all others
    monitor.enter(|val| {
        *val = 2; // this trips t_2
        Directive::NotifyAll
    });

    // eventually, all threads are woken and leave the monitor
    monitor.wait_for_num_waiting(Ordering::is_eq, 0, LONG_WAIT).unwrap();
    t_2.join().unwrap();
    t_3.join().unwrap();
    t_4.join().unwrap();
}

#[test]
fn implements_debug() {
    let monitor = SpeculativeMonitor::new("foobar");
    assert!(format!("{:?}", monitor).contains("SpeculativeMonitor"), "{:?}", monitor);
    assert!(format!("{:?}", monitor).contains("foobar"), "{:?}", monitor);

    let guard = monitor.lock();
    assert!(format!("{:?}", monitor).contains("<locked>"), "{:?}", monitor);
    drop(guard);
}

impl<T> SpeculativeMonitor<T> {
    fn wait_for_num_waiting(&self, cmp: impl FnMut(Ordering) -> bool, target: u32, duration: Duration) -> WaitResult {
        wait::Spin::wait_for_inequality(|| self.num_waiting(), cmp, &target, duration)
    }
}
