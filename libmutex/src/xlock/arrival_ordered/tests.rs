use std::cmp::Ordering;
use std::sync::{Arc, Barrier};
use std::time::Duration;
use crate::executor::{Executor, Queue, ThreadPool};
use crate::test_utils::LONG_WAIT;
use crate::remedy::Remedy;
use crate::wait;
use crate::wait::{Wait, WaitResult};
use crate::xlock::{ArrivalOrdered, XLock};

#[test]
fn readers_do_not_block_without_writer() {
    let lock = XLock::<_, ArrivalOrdered>::new(0);
    let _guard_1 = lock.read();
    let _guard_2 = lock.read();

    assert_eq!(3, lock.sync.state.lock().unwrap().next_ticket);
    assert_eq!(2, lock.sync.state.lock().unwrap().serviced_tickets);
}

#[test]
fn interleaving_writer_blocks_reader() {
    let lock = Arc::new(XLock::<_, ArrivalOrdered>::new(0));
    let guard_1 = lock.read();

    // after acquiring the lock, both the ticket and the service count should increase
    assert_eq!(2, lock.sync.state.lock().unwrap().next_ticket);
    assert_eq!(1, lock.sync.state.lock().unwrap().serviced_tickets);

    let t_2 = ThreadPool::new(1, Queue::Unbounded);

    let t_2_write = {
        let lock = lock.clone();
        t_2.submit(move || {
            lock.write();
        })
    };

    // t_2 will block trying to acquire a write lock; it should increase the next_ticket count
    lock.wait_for_next_ticket(Ordering::is_ge, 3, LONG_WAIT).unwrap();

    // the serviced_ticket count should remain
    assert_eq!(1, lock.sync.state.lock().unwrap().serviced_tickets);

    // should not be able to read-acquire
    let guard_3 = lock.try_read(Duration::ZERO);
    assert!(guard_3.is_none());

    // but timing in read-acquire should bump the serviced_tickets count
    assert_eq!(4, lock.sync.state.lock().unwrap().next_ticket);
    assert_eq!(2, lock.sync.state.lock().unwrap().serviced_tickets);

    // read-release
    drop(guard_1);

    // t_2 should eventually succeed
    assert!(t_2_write.get().is_success());
    assert_eq!(4, lock.sync.state.lock().unwrap().next_ticket);
    assert_eq!(3, lock.sync.state.lock().unwrap().serviced_tickets);

    // main can now read-acquire
    let guard_4 = lock.try_read(Duration::ZERO);
    assert!(guard_4.is_some());
    drop(guard_4);
}

#[test]
fn queuing_order() {
    let lock = Arc::new(XLock::<_, ArrivalOrdered>::new(0));
    let guard_1 = lock.read();

    // after acquiring the lock, both the ticket and the service count should increase
    assert_eq!(2, lock.sync.state.lock().unwrap().next_ticket);
    assert_eq!(1, lock.sync.state.lock().unwrap().serviced_tickets);

    let t_2 = ThreadPool::new(1, Queue::Unbounded);
    let t_3 = ThreadPool::new(1, Queue::Unbounded);
    let t_4 = ThreadPool::new(1, Queue::Unbounded);
    let t_5 = ThreadPool::new(1, Queue::Unbounded);

    let t_2_write_release = Arc::new(Barrier::new(2));
    let t_2_write = {
        let lock = lock.clone();
        let t_2_write_release = t_2_write_release.clone();
        t_2.submit(move || {
            let guard = lock.write();
            t_2_write_release.wait();
            drop(guard);
        })
    };

    // t_2 will block trying to acquire a write lock; it should increase the next_ticket count
    lock.wait_for_next_ticket(Ordering::is_ge, 3, LONG_WAIT).unwrap();
    assert_eq!(1, lock.sync.state.lock().unwrap().serviced_tickets);

    let t_3_read = {
        let lock = lock.clone();
        t_3.submit(move || {
            lock.read();
        })
    };

    // t_3 will block trying to acquire a read lock; it should increase the next_ticket count
    lock.wait_for_next_ticket(Ordering::is_ge, 4, LONG_WAIT).unwrap();
    assert_eq!(1, lock.sync.state.lock().unwrap().serviced_tickets);

    let t_4_read_release = Arc::new(Barrier::new(2));
    let t_4_read = {
        let lock = lock.clone();
        let t_4_read_release = t_4_read_release.clone();
        t_4.submit(move || {
            let guard = lock.read();
            t_4_read_release.wait();
            drop(guard);
        })
    };

    // t_4 will block trying to acquire a read lock; it should increase the next_ticket count
    lock.wait_for_next_ticket(Ordering::is_ge, 5, LONG_WAIT).unwrap();
    assert_eq!(1, lock.sync.state.lock().unwrap().serviced_tickets);

    let t_5_write = {
        let lock = lock.clone();
        t_5.submit(move || {
            lock.write();
        })
    };

    // t_5 will block trying to acquire a read lock; it should increase the next_ticket count
    lock.wait_for_next_ticket(Ordering::is_ge, 6, LONG_WAIT).unwrap();
    assert_eq!(1, lock.sync.state.lock().unwrap().serviced_tickets);

    // t_2-5 are definitely blocked
    assert!(!t_2_write.is_complete());
    assert!(!t_3_read.is_complete());
    assert!(!t_4_read.is_complete());
    assert!(!t_5_write.is_complete());

    // release the read lock -- should unblock t_2
    drop(guard_1);

    // t_3-5 remain blocked
    assert!(!t_2_write.is_complete());
    assert!(!t_3_read.is_complete());
    assert!(!t_4_read.is_complete());
    assert!(!t_5_write.is_complete());

    // tell t_2 to release the write lock
    t_2_write_release.wait();
    assert!(t_2_write.get().is_success());

    // t_3 and t_4 will unblock, acquiring read locks (t_3 terminates, while t_4 holds on)
    assert!(t_3_read.get().is_success());
    assert!(!t_5_write.is_complete());

    // tell t_4 to release the read lock
    t_4_read_release.wait();
    assert!(t_4_read.get().is_success());

    // this unblocks t_5
    assert!(t_5_write.get().is_success());
    assert_eq!(5, lock.sync.state.lock().unwrap().serviced_tickets);
}

impl<T> XLock<T, ArrivalOrdered> {
    fn wait_for_next_ticket(&self, cmp: impl FnMut(Ordering) -> bool, target: u64, duration: Duration) -> WaitResult {
        wait::Spin::wait_for_inequality(|| self.sync.state.lock().remedy().next_ticket, cmp, &target, duration)
    }
}
