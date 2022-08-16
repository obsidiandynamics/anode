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
