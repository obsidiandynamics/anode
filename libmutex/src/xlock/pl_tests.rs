//! A test suite "borrowed" from [`parking_lot`](https://github.com/Amanieu/parking_lot).

use crate::chalice::{Chalice, ChaliceResultExt};
use crate::test_utils::{SHORT_WAIT};
use rand::Rng;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use crate::xlock::locklike::LockBoxSized;
use crate::xlock::locklike::MODERATOR_KINDS;
use crate::xlock::{ArrivalOrdered, ReadBiased, WriteBiased, XLock};
use crate::xlock::UpgradeOutcome::Upgraded;

#[derive(Eq, PartialEq, Debug)]
struct NonCopy(i32);

#[test]
fn smoke() {
    for moderator in MODERATOR_KINDS {
        let l = moderator.make_lock_for_test(0);
        drop(l.read());
        drop(l.write());
        drop(l.read().upgrade());
        drop(l.write().downgrade());
    }
}

/// Enhanced over the original test to exercise both the read/write and the try_read/try_write paths,
/// as well as downgrade and try_upgrade.
#[test]
fn frob() {
    for moderator in MODERATOR_KINDS {
        const N: u32 = 10;
        const M: u32 = 1000;

        let r = Arc::new(moderator.make_lock_for_test(()));

        let (tx, rx) = channel::<()>();
        for _ in 0..N {
            let tx = tx.clone();
            let r = r.clone();
            thread::spawn(move || {
                let mut rng = rand::thread_rng();
                for _ in 0..M {
                    if rng.gen_bool(1.0 / N as f64) {
                        // println!("{t} trying write");
                        drop(r.write());
                    } else {
                        // println!("trying read");
                        drop(r.read());
                    }

                    if rng.gen_bool(1.0 / N as f64) {
                        // println!("{t} trying write");
                        drop(r.try_write(SHORT_WAIT));
                    } else {
                        // println!("trying read");
                        drop(r.try_read(SHORT_WAIT));
                    }

                    if rng.gen_bool(1.0 / N as f64) {
                        // println!("{t} trying write");
                        drop(r.try_write(Duration::ZERO));
                    } else {
                        // println!("trying read");
                        drop(r.try_read(Duration::ZERO));
                    }

                    if rng.gen_bool(1.0 / N as f64) {
                        // println!("{t} trying write");
                        drop(r.write().downgrade());
                    } else {
                        // println!("trying read");
                        if rng.gen_bool(0.5) {
                            drop(r.read().try_upgrade(SHORT_WAIT));
                        } else {
                            drop(r.read().try_upgrade(Duration::ZERO));
                        }
                    }
                }
                drop(tx);
            });
        }
        drop(tx);
        let _ = rx.recv();
    }
}

#[test]
fn test_rw_arc_no_poison_wr() {
    for moderator in MODERATOR_KINDS {
        let arc = Arc::new(moderator.make_lock_for_test(1));
        let arc2 = arc.clone();
        let _ = thread::spawn(move || {
            let _lock = arc2.write();
            panic!();
        })
        .join();
        let lock = arc.read();
        assert_eq!(*lock, 1);
    }
}

/// Like [`test_rw_arc_no_poison_wr`] , but using our fancy [`Chalice`].
#[test]
fn test_rw_arc_with_chalice() {
    for moderator in MODERATOR_KINDS {
        let arc = Arc::new(moderator.make_lock_for_test(Chalice::new(1)));
        let arc2 = arc.clone();
        let _ = thread::spawn(move || {
            let mut lock = arc2.write();
            let _val = lock.borrow_mut();
            panic!();
        })
        .join();
        let lock = arc.read();
        let res = lock.borrow();
        assert!(res.is_err());
        assert_eq!(1, *res.either());
    }
}

#[test]
fn test_rw_arc_no_poison_ww() {
    for moderator in MODERATOR_KINDS {
        let arc = Arc::new(moderator.make_lock_for_test(1));
        let arc2 = arc.clone();
        let _: Result<(), _> = thread::spawn(move || {
            let _lock = arc2.write();
            panic!();
        })
        .join();
        let lock = arc.write();
        assert_eq!(*lock, 1);
    }
}

#[test]
fn test_rw_arc_no_poison_rr() {
    for moderator in MODERATOR_KINDS {
        let arc = Arc::new(moderator.make_lock_for_test(1));
        let arc2 = arc.clone();
        let _: Result<(), _> = thread::spawn(move || {
            let _lock = arc2.read();
            panic!();
        })
        .join();
        let lock = arc.read();
        assert_eq!(*lock, 1);
    }
}

#[test]
fn test_rw_arc_no_poison_rw() {
    for moderator in MODERATOR_KINDS {
        let arc = Arc::new(moderator.make_lock_for_test(1));
        let arc2 = arc.clone();
        let _: Result<(), _> = thread::spawn(move || {
            let _lock = arc2.read();
            panic!()
        })
        .join();
        let lock = arc.write();
        assert_eq!(*lock, 1);
    }
}

/// This test had to be modified from its `parking_lot` predecessor to account for the possibility
/// of failed upgrades.
#[test]
fn test_ruw_arc() {
    for moderator in MODERATOR_KINDS {
        let arc = Arc::new(moderator.make_lock_for_test(0i32));
        let arc2 = arc.clone();
        let (tx, rx) = channel();

        // Here, a single writer will increment the value. But in the process of incrementing
        // the value, the writer will write an interim _invalid_ value: -1. The readers
        // should never observe a negative value.
        thread::spawn(move || {
            for _ in 0..10 {
                let mut lock = arc2.write();
                let tmp = *lock;
                *lock = -1;
                thread::yield_now();
                *lock = tmp + 1;
            }
            tx.send(()).unwrap();
        });

        let mut children = Vec::new();

        // Upgradable readers try to catch the writer in the act and also
        // try to touch the value. They can also write an invalid interim value.
        // Upgrading may fail, so we need to track the number of timeouts for the
        // subsequent summing assertion.
        let missed_upgrades = Arc::new(AtomicU32::new(0));
        for _ in 0..5 {
            let arc3 = arc.clone();
            let missed_upgrades = missed_upgrades.clone();
            children.push(thread::spawn(move || {
                for _ in 0..10 {
                    let lock = arc3.read();
                    let tmp = *lock;
                    assert!(tmp >= 0);
                    thread::yield_now();
                    let lock = lock.try_upgrade(SHORT_WAIT);
                    if let Upgraded(mut lock) = lock {
                        assert_eq!(tmp, *lock);
                        *lock = -1;
                        thread::yield_now();
                        *lock = tmp + 1;
                    } else {
                        missed_upgrades.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }

        // Readers try to catch the writers in the act.
        for _ in 0..5 {
            let arc4 = arc.clone();
            children.push(thread::spawn(move || {
                let lock = arc4.read();
                assert!(*lock >= 0);
            }));
        }

        // Wait for children to pass their asserts
        for r in children {
            assert!(r.join().is_ok());
        }

        // Wait for writer to finish
        rx.recv().unwrap();
        let lock = arc.read();
        let missed_upgrades = missed_upgrades.load(Ordering::Relaxed);
        assert_eq!(*lock, 60i32 - (missed_upgrades as i32));
    }
}

#[test]
fn test_rw_arc() {
    for moderator in MODERATOR_KINDS {
        let arc = Arc::new(moderator.make_lock_for_test(0));
        let arc2 = arc.clone();
        let (tx, rx) = channel();

        thread::spawn(move || {
            let mut lock = arc2.write();
            for _ in 0..10 {
                let tmp = *lock;
                *lock = -1;
                thread::yield_now();
                *lock = tmp + 1;
            }
            tx.send(()).unwrap();
        });

        // Readers try to catch the writer in the act
        let mut children = Vec::new();
        for _ in 0..5 {
            let arc3 = arc.clone();
            children.push(thread::spawn(move || {
                let lock = arc3.read();
                assert!(*lock >= 0);
            }));
        }

        // Wait for children to pass their asserts
        for r in children {
            assert!(r.join().is_ok());
        }

        // Wait for writer to finish
        rx.recv().unwrap();
        let lock = arc.read();
        assert_eq!(*lock, 10);
    }
}

#[test]
fn test_rw_arc_access_in_unwind() {
    for moderator in MODERATOR_KINDS {
        let arc = Arc::new(moderator.make_lock_for_test(1));
        let _ = {
            let arc = arc.clone();
            thread::spawn(move || {
                struct Unwinder {
                    i: Arc<LockBoxSized<usize>>,
                }
                impl Drop for Unwinder {
                    fn drop(&mut self) {
                        let mut lock = self.i.write();
                        *lock += 1;
                    }
                }
                let _u = Unwinder { i: arc };
                panic!();
            })
            .join()
        };
        let lock = arc.read();
        assert_eq!(*lock, 2);
    }
}

#[test]
fn test_rwlock_unsized() {
    let rw: &XLock<[i32], ReadBiased> = &XLock::new([1, 2, 3]);
    {
        let b = &mut *rw.write();
        b[0] = 4;
        b[2] = 5;
    }
    let comp: &[i32] = &[4, 2, 5];
    assert_eq!(&*rw.read(), comp);
}

#[test]
fn test_rwlock_try_read() {
    for moderator in MODERATOR_KINDS {
        let lock = moderator.make_lock_for_test(0isize);
        {
            let read_guard = lock.read();

            let read_result = lock.try_read(Duration::ZERO);
            assert!(
                read_result.is_some(),
                "try_read should succeed while read_guard is in scope"
            );

            drop(read_guard);
        }
        {
            let read_guard = lock.read();

            let read_result = lock.try_read(Duration::ZERO);
            assert!(
                read_result.is_some(),
                "try_read should succeed while read_guard is in scope"
            );

            drop(read_guard);
        }
        {
            let write_guard = lock.write();

            let read_result = lock.try_read(Duration::ZERO);
            assert!(
                read_result.is_none(),
                "try_read should fail while write_guard is in scope"
            );

            drop(write_guard);
        }
    }
}

#[test]
fn test_rwlock_try_write() {
    for moderator in MODERATOR_KINDS {
        let lock = moderator.make_lock_for_test(0isize);
        {
            let read_guard = lock.read();

            let write_result = lock.try_write(Duration::ZERO);
            assert!(
                write_result.is_none(),
                "try_write should fail while read_guard is in scope"
            );

            drop(read_guard);
        }
        {
            let read_guard = lock.read();

            let write_result = lock.try_write(Duration::ZERO);
            assert!(
                write_result.is_none(),
                "try_write should fail while read_guard is in scope"
            );

            drop(read_guard);
        }
        {
            let write_guard = lock.write();

            let write_result = lock.try_write(Duration::ZERO);
            assert!(
                write_result.is_none(),
                "try_write should fail while write_guard is in scope"
            );

            drop(write_guard);
        }
    }
}

/// Heavily cut down version from `parking_lot` because we don't support an exclusive upgradeable
/// reader.
#[test]
fn test_rwlock_try_upgrade() {
    for moderator in MODERATOR_KINDS {
        let lock = moderator.make_lock_for_test(0isize);
        {
            let read_guard = lock.read();

            let upgrade_result = read_guard.try_upgrade(Duration::ZERO);
            assert!(
                upgrade_result.is_upgraded(),
                "try_upgradable_read should succeed while read_guard is in scope"
            );
        }
    }
}

#[test]
fn test_into_inner() {
    for moderator in MODERATOR_KINDS {
        let m = moderator.make_lock_for_test(NonCopy(10));
        assert_eq!(m.into_inner(), NonCopy(10));
    }
}

#[test]
fn test_into_inner_drop() {
    for moderator in MODERATOR_KINDS {
        struct Foo(Arc<AtomicUsize>);
        impl Drop for Foo {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let num_drops = Arc::new(AtomicUsize::new(0));
        let m = moderator.make_lock_for_test(Foo(num_drops.clone()));
        assert_eq!(num_drops.load(Ordering::SeqCst), 0);
        {
            let _inner = m.into_inner();
            assert_eq!(num_drops.load(Ordering::SeqCst), 0);
        }
        assert_eq!(num_drops.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn test_get_mut() {
    for moderator in MODERATOR_KINDS {
        let mut m = moderator.make_lock_for_test(NonCopy(10));
        *m.get_mut() = NonCopy(20);
        assert_eq!(m.into_inner(), NonCopy(20));
    }
}

#[test]
fn test_rwlockguard_sync() {
    fn sync<T: Sync>(_: T) {}

    let rwlock = XLock::<_, ReadBiased>::new(());
    sync(rwlock.read());
    sync(rwlock.write());
}

#[test]
fn test_rwlock_downgrade() {
    for moderator in MODERATOR_KINDS {
        let x = Arc::new(moderator.make_lock_for_test(0));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let x = x.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let mut writer = x.write();
                    *writer += 1;
                    let cur_val = *writer;
                    let reader = writer.downgrade();
                    assert_eq!(cur_val, *reader);
                }
            }));
        }
        for handle in handles {
            handle.join().unwrap()
        }
        assert_eq!(*x.read(), 800);
    }
}

#[test]
fn test_rwlock_debug() {
    let x = XLock::<_, WriteBiased>::new(());
    assert!(format!("{:?}", x).contains("XLock"));
}

/// Impacts parking_lot when deadlock detection is in force. Shouldn't apply to us, but added
/// here for completeness.
#[test]
fn test_issue_203() {
    struct Bar(XLock<(), ArrivalOrdered>);

    impl Drop for Bar {
        fn drop(&mut self) {
            let _n = self.0.write();
        }
    }

    thread_local! {
        static B: Bar = Bar(XLock::new(()));
    }

    thread::spawn(|| {
        B.with(|_| ());

        let a = XLock::<_, ArrivalOrdered>::new(());
        let _a = a.read();
    })
    .join()
    .unwrap();
}
