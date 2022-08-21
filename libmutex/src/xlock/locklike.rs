use crate::xlock::{ArrivalOrdered, LockReadGuard, LockWriteGuard, ReadBiased, Spec, UpgradeOutcome, WriteBiased, XLock};
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

#[derive(Debug)]
pub enum ModeratorKind {
    ReadBiased,
    WriteBiased,
    ArrivalOrdered
}

pub const MODERATOR_KINDS: [ModeratorKind; 3] = [
    ModeratorKind::ReadBiased,
    ModeratorKind::WriteBiased,
    ModeratorKind::ArrivalOrdered,
];

impl ModeratorKind {
    pub fn lock_for_test<T: 'static>(&self, t: T) -> LockBox<T> {
        println!("test running with moderator {:?}", self);
        match self {
            ModeratorKind::ReadBiased => Box::new(XLock::<_, ReadBiased>::new(t)),
            ModeratorKind::WriteBiased => Box::new(XLock::<_, WriteBiased>::new(t)),
            ModeratorKind::ArrivalOrdered => Box::new(XLock::<_, ArrivalOrdered>::new(t)),
        }
    }
}