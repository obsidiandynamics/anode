use std::cmp::Ordering;
use std::sync::Arc;
use std::time::Duration;
use test_utils::SHORT_WAIT;
use crate::executor::{Executor, Queue, ThreadPool};
use crate::{test_utils, wait};
use crate::monitor::{Monitor};
use crate::test_utils::LONG_WAIT;
use crate::wait::{Wait, WaitResult};
use crate::xlock::{WriteBiased, XLock};

#[test]
fn timeout_in_write_unblocks_readers() {
    let lock = XLock::<_, WriteBiased>::new(0);
    let guard_1 = lock.read();
    let guard_2 = lock.read();

    // cannot acquire a write lock with active readers
    let guard_3 = lock.try_write(SHORT_WAIT);
    assert!(guard_3.is_none());

    // the timeout should have cleared the writer_pending flag
    assert!(!lock.is_writer_pending());
    let guard_4 = lock.read();
    drop(guard_1);
    drop(guard_2);
    drop(guard_4);
}

#[test]
fn timeout_in_upgrade_unblocks_readers() {
    let lock = XLock::<_, WriteBiased>::new(0);
    let guard_1 = lock.read();
    let guard_2 = lock.read();

    // cannot acquire upgrade a read lock with active readers
    let guard_3 = lock.read().try_upgrade(SHORT_WAIT);
    assert!(guard_3.is_unchanged());

    // the timeout should have cleared the writer_pending flag
    assert!(!lock.is_writer_pending());
    let guard_4 = lock.read();
    drop(guard_1);
    drop(guard_2);
    drop(guard_4);
}

#[test]
fn await_pending_writer() {
    let lock = Arc::new(XLock::<_, WriteBiased>::new(0));
    let guard_1 = lock.read();

    let t_2 = ThreadPool::new(1, Queue::Unbounded);
    let t_2_write = {
        let lock = lock.clone();
        t_2.submit(move || {
            println!("t_2 waiting for write");
            let guard_2 = lock.write();
            println!("t_2 write-acquired");
            drop(guard_2);
            println!("t_2 exiting");
        })
    };

    // t_2 cannot write-acquire while main holds the read lock
    assert!(!t_2_write.is_complete());

    // wait until we are sure that t_2 has raised the writer_pending flag
    lock.wait_for_writer_pending_flag(true, LONG_WAIT).unwrap();

    // try-read a second time from main; should fail, since there is a pending writer
    println!("main waiting for read #2");
    let guard_3 = lock.try_read(SHORT_WAIT);
    assert!(guard_3.is_none());

    // release the read lock; this will unblock t_2
    println!("main read-release");
    drop(guard_1);

    // wait until t_2 acquires the lock and releases it
    assert!(t_2_write.get().is_success());

    // t_2 should have cleared the writer_pending flag
    assert!(!lock.is_writer_pending());

    // acquiring the read lock will succeed
    println!("main waiting for read #3");
    let guard_4 = lock.try_read(Duration::ZERO);
    assert!(guard_4.is_some());
    drop(guard_4);
}

#[test]
fn await_pending_writer_timeout() {
    let lock = Arc::new(XLock::<_, WriteBiased>::new(0));
    let guard_1 = lock.read();

    let t_2 = ThreadPool::new(1, Queue::Unbounded);
    let t_2_write = {
        let lock = lock.clone();
        t_2.submit(move || {
            let guard_2 = lock.try_write(SHORT_WAIT);
            assert!(guard_2.is_none());
        })
    };

    // wait until t_2 exits
    assert!(t_2_write.get().is_success());

    // after timing out on try_write, t_2 should have cleared the writer_pending flag
    assert!(!lock.is_writer_pending());

    let guard_4 = lock.try_read(Duration::ZERO);
    assert!(guard_4.is_some());
    drop(guard_4);

    drop(guard_1);
}

impl<T> XLock<T, WriteBiased> {
    fn is_writer_pending(&self) -> bool {
        self.sync.monitor.compute(|state| state.writer_pending)
    }

    fn wait_for_writer_pending_flag(&self, target: bool, duration: Duration) -> WaitResult {
        wait::Spin::wait_for_inequality(|| self.is_writer_pending(), Ordering::is_eq, &target, duration)
    }
}