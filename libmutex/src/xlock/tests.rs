use std::time::{Duration};
use crate::xlock::locklike::{LockBoxSized, LockReadGuardlike, LockWriteGuardlike, MODERATOR_KINDS};

#[test]
fn box_cycle() {
    for moderator in MODERATOR_KINDS {
        __box_cycle(moderator.make_lock_for_test(42));
    }
}

fn __box_cycle(boxed: LockBoxSized<i32>) {
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
