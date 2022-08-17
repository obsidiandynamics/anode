//! A test suite "borrowed" from [TransRAM](https://github.com/obsidiandynamics/transram).

use std::sync::{Arc, Barrier};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use crate::test_utils;
use crate::test_utils::{CHECK_WAIT, LONG_WAIT, SHORT_WAIT};
use super::UnfairLock;

#[test]
fn read_release_cycle() {
    let lock = UnfairLock::new(0);
    for _ in 0..3 {
        let guard = lock.read();
        assert_eq!(0, *guard);
        drop(guard);
    }
    assert_eq!(0, lock.into_inner());
}

#[test]
fn read_upgrade_release_cycle() {
    let lock = UnfairLock::new(0);
    let cycles = 3;
    for i in 0..cycles {
        let guard = lock.read();
        assert_eq!(i, *guard);
        let mut guard = guard.upgrade();
        assert_eq!(i, *guard);
        *guard += 1;
    }
    assert_eq!(cycles, lock.into_inner());
}

#[test]
fn read_upgrade_downgrade_release_cycle() {
    let lock = UnfairLock::new(0);
    let cycles = 3;
    for i in 0..cycles {
        let guard = lock.read();
        assert_eq!(i, *guard);
        let mut guard = guard.upgrade();
        assert_eq!(i, *guard);
        *guard += 1;
        let guard = guard.downgrade();
        assert_eq!(i + 1, *guard);
    }
    assert_eq!(cycles, lock.into_inner());
}

#[test]
fn write_release_cycle() {
    let lock = UnfairLock::new(0);
    let cycles = 3;
    for i in 0..cycles {
        let mut guard = lock.write();
        assert_eq!(i, *guard);
        *guard += 1;
    }
    assert_eq!(cycles, lock.into_inner());
}

#[test]
fn write_downgrade_release_cycle() {
    let lock = UnfairLock::new(0);
    let cycles = 3;
    for i in 0..cycles {
        let mut guard = lock.write();
        assert_eq!(i, *guard);
        *guard += 1;
        let guard = guard.downgrade();
        assert_eq!(i + 1, *guard);
    }
    assert_eq!(cycles, lock.into_inner());
}

#[test]
fn write_downgrade_upgrade_release_cycle() {
    let lock = UnfairLock::new(0);
    let cycles = 3;
    for i in 0..cycles {
        let mut guard = lock.write();
        assert_eq!(i * 2, *guard);
        *guard += 1;
        let guard = guard.downgrade();
        assert_eq!(i * 2 + 1, *guard);
        let mut guard = guard.upgrade();
        assert_eq!(i * 2 + 1, *guard);
        *guard += 1;
    }
    assert_eq!(cycles * 2, lock.into_inner());
}

#[test]
fn read_acquire_while_read_locked() {
    let lock = UnfairLock::new(0);
    let guard_1 = lock.read();
    let guard_2 = lock.read();
    drop(guard_1);
    drop(guard_2);
}

#[test]
fn timeout_on_write_acquire_while_read_locked() {
    let lock = UnfairLock::new(0);
    let guard_1 = lock.read();
    let guard_2_res = lock.try_write(SHORT_WAIT);
    assert!(guard_2_res.is_none());
    let guard_2_res = lock.try_write(Duration::ZERO);
    assert!(guard_2_res.is_none());
    drop(guard_1);
    let guard_2_res = lock.try_write(Duration::ZERO);
    assert!(guard_2_res.is_some());
}

#[test]
fn timeout_on_upgrade_while_read_locked() {
    let lock = UnfairLock::new(0);
    let guard_1 = lock.read();
    let guard_2 = lock.read();
    let guard_2_res = guard_2.try_upgrade(SHORT_WAIT);
    assert!(guard_2_res.is_unchanged());
    let guard_2 = guard_2_res.unchanged().unwrap();
    let guard_2_res = guard_2.try_upgrade(Duration::ZERO);
    assert!(guard_2_res.is_unchanged());
    let guard_2 = guard_2_res.unchanged().unwrap();
    drop(guard_1);
    let guard_2_res = guard_2.try_upgrade(Duration::ZERO);
    assert!(guard_2_res.is_upgraded());
}

#[test]
fn timeout_on_write_acquire_while_write_locked() {
    let lock = UnfairLock::new(0);
    let guard_1 = lock.write();
    let guard_2_res = lock.try_write(SHORT_WAIT);
    assert!(guard_2_res.is_none());
    let guard_2_res = lock.try_write(Duration::ZERO);
    assert!(guard_2_res.is_none());
    drop(guard_1);
    let guard_2_res = lock.try_write(Duration::ZERO);
    assert!(guard_2_res.is_some());
}

#[test]
fn timeout_on_read_acquire_while_write_locked() {
    let lock = UnfairLock::new(0);
    let guard_1 = lock.write();
    let guard_2_res = lock.try_read(SHORT_WAIT);
    assert!(guard_2_res.is_none());
    let guard_2_res = lock.try_read(Duration::ZERO);
    assert!(guard_2_res.is_none());
    drop(guard_1);
    let guard_2_res = lock.try_read(Duration::ZERO);
    assert!(guard_2_res.is_some());
}

#[test]
fn await_write_acquire_while_read_locked() {
    let lock = Arc::new(UnfairLock::new(0));
    let guard_1 = lock.read();

    let lock_t_2 = lock.clone();
    let t_2 = test_utils::spawn_blocked(move || {
        // t_2 blocks because main holds the read lock
        let guard_2_res = lock_t_2.try_write(LONG_WAIT);
        guard_2_res.is_some()
    });
    assert!(!t_2.is_finished());

    // t_2 should be unblocked after main releases the read lock
    drop(guard_1);
    assert!(t_2.join().unwrap());
}

#[test]
fn await_write_acquire_while_locked_by_several_readers() {
    let lock = Arc::new(UnfairLock::new(0));
    let guard_1 = lock.read();
    let guard_2 = lock.read();

    let lock_t_3 = lock.clone();
    let t_3 = test_utils::spawn_blocked(move || {
        // t_3 blocks because main holds the read lock (twice)
        let guard_3_res = lock_t_3.try_write(LONG_WAIT);
        guard_3_res.is_some()
    });
    assert!(!t_3.is_finished());

    // t_3 should be unblocked after main releases all read locks
    drop(guard_1);
    thread::sleep(CHECK_WAIT);
    assert!(!t_3.is_finished());

    drop(guard_2);
    assert!(t_3.join().unwrap());
}

#[test]
fn await_upgrade_acquire_while_read_locked() {
    let lock = Arc::new(UnfairLock::new(0));
    let guard_1 = lock.read();

    let lock_t_2 = lock.clone();
    let t_2 = test_utils::spawn_blocked(move || {
        // t_2 starts by acquiring its own read lock
        let guard_2 = lock_t_2.read();
        // t_2 blocks because main holds the read lock (its own read lock will not affect it)
        let guard_2_res = guard_2.try_upgrade(LONG_WAIT);
        guard_2_res.is_upgraded()
    });
    assert!(!t_2.is_finished());

    // t_2 should be unblocked after main releases the read lock
    drop(guard_1);
    assert!(t_2.join().unwrap());
}

#[test]
fn await_upgrade_acquire_while_locked_by_several_readers() {
    let lock = Arc::new(UnfairLock::new(0));
    let guard_1 = lock.read();
    let guard_2 = lock.read();

    let lock_t_3 = lock.clone();
    let t_3 = test_utils::spawn_blocked(move || {
        // t_3 starts by acquiring its own read lock
        let guard_3 = lock_t_3.read();
        // t_3 blocks because main holds the read lock (twice)
        let guard_3_res = guard_3.try_upgrade(LONG_WAIT);
        guard_3_res.is_upgraded()
    });
    assert!(!t_3.is_finished());

    // t_3 should be unblocked after main releases all read locks
    drop(guard_1);
    thread::sleep(CHECK_WAIT);
    assert!(!t_3.is_finished());

    drop(guard_2);
    assert!(t_3.join().unwrap());
}

#[test]
fn await_read_acquire_while_write_locked() {
    let lock = Arc::new(UnfairLock::new(0));
    let guard_1 = lock.write();

    let lock_t_2 = lock.clone();
    let t_2 = test_utils::spawn_blocked(move || {
        // t_2 blocks because main holds the write lock
        let guard_2_res = lock_t_2.try_read(LONG_WAIT);
        guard_2_res.is_some()
    });
    assert!(!t_2.is_finished());

    // t_2 should be unblocked after main releases the write lock
    drop(guard_1);
    assert!(t_2.join().unwrap());
}

#[test]
fn await_read_acquire_while_write_locked_with_downgrade() {
    let lock = Arc::new(UnfairLock::new(0));
    let guard_1 = lock.write();

    let lock_t_2 = lock.clone();
    let t_2 = test_utils::spawn_blocked(move || {
        // t_2 blocks because main holds the write lock
        let guard_2_res = lock_t_2.try_read(LONG_WAIT);
        guard_2_res.is_some()
    });
    assert!(!t_2.is_finished());

    // t_2 should be unblocked after main downgrades the write lock
    let guard_1 = guard_1.downgrade();
    assert!(t_2.join().unwrap());
    drop(guard_1);
}

#[test]
fn competing_read_acquire_and_upgrade_while_read_locked() {
    let lock = Arc::new(UnfairLock::new(0));
    let guard_1 = lock.read();
    println!("main: read-acquired");

    let read_acquired = Arc::new(Barrier::new(2));
    let t_2_upgraded = Arc::new(Barrier::new(2));
    let t_2_begin_downgrade = Arc::new(Barrier::new(2));
    let t_2_downgraded = Arc::new(Barrier::new(2));
    let t_2_begin_release = Arc::new(Barrier::new(2));

    let t_2 = {
        let lock = lock.clone();
        let read_acquired = read_acquired.clone();
        let t_2_upgraded = t_2_upgraded.clone();
        let t_2_begin_downgrade = t_2_begin_downgrade.clone();
        let t_2_downgraded = t_2_downgraded.clone();
        let t_2_begin_release = t_2_begin_release.clone();
        thread::spawn(move || {
            let guard_2_res = lock.try_read(LONG_WAIT);
            assert!(guard_2_res.is_some());
            let guard_2 = guard_2_res.unwrap();
            println!("t_2: read-acquired");
            read_acquired.wait();

            println!("t_2: trying upgrade");
            let guard_2_res = guard_2.try_upgrade(LONG_WAIT);
            assert!(guard_2_res.is_upgraded());
            let guard_2 = guard_2_res.upgraded().unwrap();
            println!("t_2: upgraded");
            t_2_upgraded.wait();

            t_2_begin_downgrade.wait();
            println!("t_2: downgrading");
            let guard_2 = guard_2.downgrade();
            t_2_downgraded.wait();

            t_2_begin_release.wait();
            drop(guard_2);
            println!("t_2: exiting");
        })
    };

    let t_3 = {
        let lock = lock.clone();
        thread::spawn(move || {
            let guard_3_res = lock.try_write(LONG_WAIT);
            assert!(guard_3_res.is_some());
            println!("t_3: write-acquired");
            drop(guard_3_res);
            println!("t_3: exiting");
        })
    };

    read_acquired.wait();
    // main and t_2 should be in a read-acquired state; t_2 and t_3 should be blocked
    thread::sleep(CHECK_WAIT);
    assert!(!t_2.is_finished());
    assert!(!t_3.is_finished());

    // release main; t_2 should upgrade successfully
    println!("main: read-releasing");
    drop(guard_1);
    t_2_upgraded.wait();

    // t_3 remains blocked
    assert!(!t_3.is_finished());

    // tell t_2 to downgrade its lock
    t_2_begin_downgrade.wait();
    t_2_downgraded.wait();

    // t_3 remains blocked
    assert!(!t_3.is_finished());

    // tell t_2 to read-release its lock
    t_2_begin_release.wait();
    t_2.join().unwrap();
    // t_3 can now write-acquire (and exit)
    t_3.join().unwrap();
}

#[test]
fn competing_multiple_write_acquire_while_read_locked() {
    let lock = Arc::new(UnfairLock::new(0));
    let guard_1 = lock.read();
    println!("main: read-acquired");

    let write_acquired = Arc::new(Barrier::new(2));
    let t_2_write_acquired = Arc::new(AtomicBool::default());
    let t_3_write_acquired = Arc::new(AtomicBool::default());
    let begin_release = Arc::new(Barrier::new(2));

    let t_2 = {
        let lock = lock.clone();
        let write_acquired = write_acquired.clone();
        let t_2_write_acquired = t_2_write_acquired.clone();
        let begin_release = begin_release.clone();
        test_utils::spawn_blocked(move || {
            println!("t_2: trying write-acquire");
            let guard_2_res = lock.try_write(LONG_WAIT);
            assert!(guard_2_res.is_some());
            println!("t_2: write-acquired");
            t_2_write_acquired.store(true, Ordering::Relaxed);
            write_acquired.wait();
            begin_release.wait();
            println!("t_2: exiting");
        })
    };

    let t_3 = {
        let lock = lock.clone();
        let write_acquired = write_acquired.clone();
        let t_3_write_acquired = t_3_write_acquired.clone();
        let begin_release = begin_release.clone();
        test_utils::spawn_blocked(move || {
            println!("t_3: trying write-acquire");
            let guard_3_res = lock.try_write(LONG_WAIT);
            assert!(guard_3_res.is_some());
            println!("t_3: write-acquired");
            t_3_write_acquired.store(true, Ordering::Relaxed);
            write_acquired.wait();
            begin_release.wait();
            println!("t_3: exiting");
        })
    };

    // t_2 and t_3 will initially be blocked, waiting for write-acquire
    assert!(!t_2.is_finished());
    assert!(!t_3.is_finished());

    // drop read lock and wait for either t_2 or t_3 to write-acquire
    println!("main: read-releasing");
    drop(guard_1);
    write_acquired.wait();

    // exactly one of t_2 or t_3 will have the write lock
    let t_2_write_acquired = t_2_write_acquired.load(Ordering::Relaxed);
    let t_3_write_acquired = t_3_write_acquired.load(Ordering::Relaxed);
    assert_ne!(t_2_write_acquired, t_3_write_acquired);

    if t_2_write_acquired {
        assert!(!t_3.is_finished());
        begin_release.wait();
        t_2.join().unwrap();

        // these two barriers are reused; we need to trip them again for t_3 to progress
        write_acquired.wait();
        begin_release.wait();

        t_3.join().unwrap();
    } else {
        assert!(!t_2.is_finished());
        begin_release.wait();
        t_3.join().unwrap();

        // these two barriers are reused; we need to trip them again for t_2 to progress
        write_acquired.wait();
        begin_release.wait();

        t_2.join().unwrap();
    }
}

// @Test
// void testCompetingMultipleWriteAcquireWhileReadLocked() throws InterruptedException {
// final var mutex = new UnfairUpgradeableMutex();
// final var m1 = threaded(mutex);
// final var m2 = threaded(mutex);
// final var m3 = threaded(mutex);
// assertThat(m1.tryReadAcquire(Long.MAX_VALUE)).isTrue();
// final var m2_tryWriteAcquire = m2.tryWriteAcquireAsync(Long.MAX_VALUE);
// final var m3_tryWriteAcquire = m3.tryWriteAcquireAsync(Long.MAX_VALUE);
//
// // neither m2 nor m3 may proceed initially
// Thread.sleep(SHORT_SLEEP_MS);
// assertThat(m2_tryWriteAcquire.completable().isDone()).isFalse();
// assertThat(m3_tryWriteAcquire.completable().isDone()).isFalse();
//
// m1.readRelease();
// // after read-release, exactly one of m2 or m3 will unblock
// CompletableFuture.anyOf(m2_tryWriteAcquire.completable(), m3_tryWriteAcquire.completable()).join();
// Thread.sleep(SHORT_SLEEP_MS);
// assertThat(m2_tryWriteAcquire.completable().isDone() ^ m3_tryWriteAcquire.completable().isDone()).isTrue();
// if (m2_tryWriteAcquire.completable().isDone()) {
// assertThat(m2_tryWriteAcquire.get()).isTrue();
// m2.writeRelease();
// assertThat(m3_tryWriteAcquire.get()).isTrue();
// } else {
// assertThat(m3_tryWriteAcquire.get()).isTrue();
// m3.writeRelease();
// assertThat(m2_tryWriteAcquire.get()).isTrue();
// }
// }
// }