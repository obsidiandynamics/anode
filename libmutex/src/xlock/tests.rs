use crate::xlock::{LockReadGuard, LockWriteGuard, ReadBiased, Spec, UpgradeOutcome, XLock};
use std::ops::{Deref, DerefMut};
use std::time::Duration;

pub type LockBox<T> = Box<dyn Locklike<T>>;

pub type LockBoxSized<T> = Box<dyn LocklikeSized<T>>;

pub type DynLockUpgradeOutcome<'a, T> =
    UpgradeOutcome<DynLockWriteGuard<'a, T>, DynLockReadGuard<'a, T>>;

pub trait Locklike<T: ?Sized> {
    fn read(&self) -> DynLockReadGuard<T>;

    fn try_read(&self, duration: Duration) -> Option<DynLockReadGuard<T>>;

    fn write(&self) -> DynLockWriteGuard<T>;

    fn try_write(&self, duration: Duration) -> Option<DynLockWriteGuard<T>>;

    fn get_mut(&mut self) -> &mut T;
}

pub trait LocklikeSized<T>: Locklike<T> {
    fn into_inner(self: Box<Self>) -> T;
}

trait LockReadGuardlike<'a, T: ?Sized>: Deref<Target = T> {
    fn upgrade_box(self: Box<Self>) -> DynLockWriteGuard<'a, T>;

    fn try_upgrade_box(self: Box<Self>, duration: Duration) -> DynLockUpgradeOutcome<'a, T>;
}

impl<'a, T: ?Sized, S: Spec> LockReadGuardlike<'a, T> for LockReadGuard<'a, T, S> {
    fn upgrade_box(self: Box<Self>) -> DynLockWriteGuard<'a, T> {
        self.upgrade().into()
    }

    fn try_upgrade_box(self: Box<Self>, duration: Duration) -> DynLockUpgradeOutcome<'a, T> {
        self.try_upgrade(duration)
            .map(DynLockWriteGuard::from, DynLockReadGuard::from)
    }
}

pub struct DynLockReadGuard<'a, T: ?Sized>(Box<dyn LockReadGuardlike<'a, T> + 'a>);

impl<'a, T: ?Sized> DynLockReadGuard<'a, T> {
    pub fn upgrade(self) -> DynLockWriteGuard<'a, T> {
        self.0.upgrade_box()
    }

    pub fn try_upgrade(self, duration: Duration) -> DynLockUpgradeOutcome<'a, T> {
        self.0.try_upgrade_box(duration)
    }
}

impl<T: ?Sized> Deref for DynLockReadGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        self.0.as_ref()
    }
}

impl<'a, T: ?Sized, S: Spec> From<LockReadGuard<'a, T, S>> for DynLockReadGuard<'a, T> {
    fn from(guard: LockReadGuard<'a, T, S>) -> Self {
        DynLockReadGuard(Box::new(guard))
    }
}

trait LockWriteGuardlike<'a, T: ?Sized>: DerefMut<Target = T> {
    fn downgrade_box(self: Box<Self>) -> DynLockReadGuard<'a, T>;
}

impl<'a, T: ?Sized, S: Spec> LockWriteGuardlike<'a, T> for LockWriteGuard<'a, T, S> {
    fn downgrade_box(self: Box<Self>) -> DynLockReadGuard<'a, T> {
        self.downgrade().into()
    }
}

pub struct DynLockWriteGuard<'a, T: ?Sized>(Box<dyn LockWriteGuardlike<'a, T> + 'a>);

impl<'a, T: ?Sized> DynLockWriteGuard<'a, T> {
    pub fn downgrade(self) -> DynLockReadGuard<'a, T> {
        self.0.downgrade_box().into()
    }
}

impl<T: ?Sized> Deref for DynLockWriteGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        self.0.as_ref()
    }
}

impl<T: ?Sized> DerefMut for DynLockWriteGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.0.as_mut()
    }
}

impl<'a, T: ?Sized, S: Spec> From<LockWriteGuard<'a, T, S>> for DynLockWriteGuard<'a, T> {
    fn from(guard: LockWriteGuard<'a, T, S>) -> Self {
        DynLockWriteGuard(Box::new(guard))
    }
}

impl<T: ?Sized, S: Spec> Locklike<T> for XLock<T, S> {
    fn read(&self) -> DynLockReadGuard<T> {
        DynLockReadGuard(Box::new(self.read()))
    }

    fn try_read(&self, duration: Duration) -> Option<DynLockReadGuard<T>> {
        self.try_read(duration).map(DynLockReadGuard::from)
    }

    fn write(&self) -> DynLockWriteGuard<T> {
        DynLockWriteGuard(Box::new(self.write()))
    }

    fn try_write(&self, duration: Duration) -> Option<DynLockWriteGuard<T>> {
        self.try_write(duration).map(DynLockWriteGuard::from)
    }

    fn get_mut(&mut self) -> &mut T {
        self.get_mut()
    }
}


impl<T, S: Spec> XLock<T, S> {
    fn lock_into_inner(self) -> T {
        self.into_inner()
    }
}

impl<T, S: Spec> LocklikeSized<T> for XLock<T, S> {
    fn into_inner(self: Box<Self>) -> T {
        self.lock_into_inner()
    }
}

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
