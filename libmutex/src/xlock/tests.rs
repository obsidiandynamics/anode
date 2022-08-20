use std::time::Duration;
use crate::xlock::locklike::{LockBox, LockBoxSized};
use crate::xlock::{ReadBiased, XLock};

#[test]
fn box_cycle() {
    let lock = XLock::<_, ReadBiased>::new(42);
    let boxed: LockBox<_> = Box::new(lock);

    // read -> release
    {
        let guard = boxed.read();
        assert_eq!(42, *guard);
    }

    // read -> upgrade -> downgrade -> release
    {
        let mut guard = boxed.read().upgrade();
        assert_eq!(42, *guard);
        *guard = 1911;
        assert_eq!(1911, *guard);

        let guard = guard.downgrade();
        assert_eq!(1911, *guard);
    }

    // write -> release
    {
        let mut guard = boxed.write();
        assert_eq!(1911, *guard);
        *guard = 1801;
        assert_eq!(1801, *guard);
    }

    // write -> downgrade -> release
    {
        let mut guard = boxed.write();
        assert_eq!(1801, *guard);
        *guard = 69;
        assert_eq!(69, *guard);

        let guard = guard.downgrade(); // drops old guard
        assert_eq!(69, *guard);
    }

    // try-read -> upgrade -> downgrade -> try_upgrade -> downgrade -> release
    {
        let guard = boxed.try_read(Duration::ZERO).unwrap();
        assert_eq!(69, *guard);

        let mut guard = guard.upgrade(); // drops old guard
        *guard = 1945;
        assert_eq!(1945, *guard);

        let guard = guard.downgrade(); // drops old guard
        assert_eq!(1945, *guard);

        let guard = guard.try_upgrade(Duration::ZERO); // possibly drops old guard
        assert!(guard.is_upgraded()); // in this case, definitely drops
        let mut guard = guard.upgraded().unwrap();
        *guard = 1941;
        assert_eq!(1941, *guard);

        let guard = guard.downgrade(); // drops old guard
        assert_eq!(1941, *guard);
    }

    // try-write -> downgrade -> release
    {
        let mut guard = boxed.try_write(Duration::ZERO).unwrap();
        *guard = 1983;
        assert_eq!(1983, *guard);

        let guard = guard.downgrade(); // drops old guard
        assert_eq!(1983, *guard);
    }
}

#[test]
fn box_sized_into_inner() {
    let lock = XLock::<_, ReadBiased>::new(42);
    let boxed: LockBoxSized<_> = Box::new(lock);
    assert_eq!(42, boxed.into_inner());
}
