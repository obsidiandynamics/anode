use test_utils::SHORT_WAIT;
use crate::multilock::{Fairness, MultiLock};
use crate::test_utils;

#[test]
fn timeout_in_write_unblocks_readers() {
    let lock = MultiLock::new(0, Fairness::WriteBiased);
    let guard_1 = lock.read();
    let guard_2 = lock.read();

    // cannot acquire a write lock with active readers
    let guard_3 = lock.try_write(SHORT_WAIT);
    assert!(guard_3.is_none());

    // the timeout should have cleared the writer_pending flag
    let guard_4 = lock.read();
    drop(guard_1);
    drop(guard_2);
    drop(guard_4);
}


#[test]
fn timeout_in_upgrade_unblocks_readers() {
    let lock = MultiLock::new(0, Fairness::WriteBiased);
    let guard_1 = lock.read();
    let guard_2 = lock.read();

    // cannot acquire upgrade a read lock with active readers
    let guard_3 = lock.read().try_upgrade(SHORT_WAIT);
    assert!(guard_3.is_unchanged());

    // the timeout should have cleared the writer_pending flag
    let guard_4 = lock.read();
    drop(guard_1);
    drop(guard_2);
    drop(guard_4);
}

// #[test]
// fn await_timeout_in_pending_writer() {
//     let lock = Arc::new(MultiLock::new(0, Fairness::WriteBiased));
//     let guard_1 = lock.read();
//
// }