use std::sync::{Arc, Barrier};
use crate::spin_mutex::SpinMutex;
use crate::test_utils;

#[test]
fn cycle() {
    let lock = SpinMutex::new(0);
    let mut guard_1 = lock.lock();
    assert_eq!(0, *guard_1);
    *guard_1 = 42;

    let guard_2 = lock.try_lock();
    assert!(guard_2.is_none());
    drop(guard_1);

    let mut guard_3 = lock.try_lock().unwrap();
    assert_eq!(42, *guard_3);
    *guard_3 = 69;
    assert_eq!(69, *guard_3);
    drop(guard_3);
}

#[test]
fn borrow_mut() {
    let mut lock = SpinMutex::new(0);
    let val  = lock.get_mut();
    *val = 42;

    let guard = lock.lock();
    assert_eq!(42, *guard);
}

#[test]
fn into_inner() {
    let lock = SpinMutex::new(0);
    let mut guard = lock.lock();
    assert_eq!(0, *guard);
    *guard = 42;
    drop(guard);

    assert_eq!(42, lock.into_inner());
}

#[test]
fn await_release() {
    for _ in 0..10 {
        let lock = Arc::new(SpinMutex::new(0));
        let mut guard_1 = lock.lock();
        *guard_1 = 42;

        let t_2_before_acquire = Arc::new(Barrier::new(2));
        let t_2_acquired = Arc::new(Barrier::new(2));
        let t_2 = {
            let lock = lock.clone();
            let t_2_before_acquire = t_2_before_acquire.clone();
            let t_2_acquired = t_2_acquired.clone();
            test_utils::spawn_blocked(move || {
                // cannot acquire a lock -- it's held by main
                assert!(lock.try_lock().is_none());

                // block until main releases the lock
                t_2_before_acquire.wait();
                let mut guard_2 = lock.lock();
                t_2_acquired.wait();
                assert_eq!(42, *guard_2);
                *guard_2 = 69;
            })
        };

        // t_2 is still blocked and the value hasn't changed
        assert!(!t_2.is_finished());
        assert_eq!(42, *guard_1);

        // unlock from main and let t_2 run to completion
        t_2_before_acquire.wait();
        drop(guard_1);
        t_2_acquired.wait();

        let guard_3 = lock.lock();
        assert_eq!(69, *guard_3);
    }
}

#[test]
fn debug() {
    let lock = SpinMutex::new("foobar");
    assert!(format!("{:?}", lock).contains("SpinMutex"), "{:?}", lock);
    assert!(format!("{:?}", lock).contains("foobar"), "{:?}", lock);

    let guard = lock.lock();
    assert!(format!("{:?}", lock).contains("<locked>"), "{:?}", lock);
    drop(guard);
}