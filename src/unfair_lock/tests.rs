use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;
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

const SHORT_WAIT: Duration = Duration::from_micros(1);
const LONG_WAIT: Duration = Duration::from_secs(1);

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
    println!("outer got guard {:?}", guard_1);

    let lock_t_2 = lock.clone();
    let barrier = Arc::new(Barrier::new(2));
    let barrier_t_2 = barrier.clone();
    let t_2 = thread::spawn(move || {
        barrier_t_2.wait();
        let guard_2_res = lock_t_2.try_write(LONG_WAIT);
        println!("inner got guard {:?}", guard_2_res);
        guard_2_res.is_some()
    });
    barrier.wait(); // wait for thread t_2 to start
    assert!(!t_2.is_finished());

    println!("outer guard is {:?}", guard_1);
    drop(guard_1);
    assert!(t_2.join().unwrap());
}

// @Test
// void testAwaitWriteAcquireWhileReadLocked() throws InterruptedException {
// final var mutex = new UnfairUpgradeableMutex();
// final var m1 = threaded(mutex);
// final var m2 = threaded(mutex);
// assertThat(m1.tryReadAcquire(Long.MAX_VALUE)).isTrue();
// final var m2_tryWriteAcquire = m2.tryWriteAcquireAsync(Long.MAX_VALUE);
// Thread.sleep(SHORT_SLEEP_MS);
// assertThat(m2_tryWriteAcquire.completable().isDone()).isFalse();
// m1.readRelease();
// assertThat(m2_tryWriteAcquire.get()).isTrue();
// }