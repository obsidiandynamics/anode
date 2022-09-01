use std::sync::Arc;
use std::thread;
use std::time::Duration;
use anode::zlock::{ReadBiased, ZLock};
use crate::lock_spec::LockSpec;

#[test]
fn conformance() {
    let lock = ZLock::<_, ReadBiased>::new(0);
    takes_borrowed(&lock);

    takes_owned(lock);

    takes_owned_alt(ZLock::<_, ReadBiased>::new(0));
}

fn takes_borrowed<'a, L: LockSpec<'a, T=u64>>(lock: &'a L) {
    let guard = lock.try_read(Duration::ZERO).unwrap();
    assert_eq!(0, *guard);

    let mut guard = L::try_upgrade(guard, Duration::ZERO).upgraded().unwrap();
    assert_eq!(0, *guard);
    *guard = 42;

    let guard = L::downgrade(guard);
    assert_eq!(42, *guard);

    drop(guard);

    let mut guard = lock.try_write(Duration::ZERO).unwrap();
    assert_eq!(42, *guard);
    *guard = 69;

    let guard = L::downgrade(guard);
    assert_eq!(69, *guard);
}

fn takes_owned<L>(lock: L)
    where for <'a> L: LockSpec<'a, T=u64> + 'static
{
    let arc = Arc::new(lock);
    thread::spawn(move || {
        arc.try_read(Duration::ZERO);
    }).join().unwrap();
}

fn takes_owned_alt<L: for <'a> LockSpec<'a, T=u64> + 'static>(lock: L) {
    let arc = Arc::new(lock);
    thread::spawn(move || {
        arc.try_read(Duration::ZERO);
    }).join().unwrap();
}