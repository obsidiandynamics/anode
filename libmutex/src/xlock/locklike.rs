use std::any::Any;
use crate::xlock::{ArrivalOrdered, LockReadGuard, LockWriteGuard, ReadBiased, Moderator, UpgradeOutcome, WriteBiased, XLock};
use std::ops::{Deref, DerefMut};
use std::time::Duration;

pub type LockBox<'b, T> = Box<dyn Locklike<T, R<'b> = DynLockReadGuard<'b, T>, W<'b> = DynLockWriteGuard<'b, T>>>;

pub type LockBoxSized<'b, T> = Box<dyn LocklikeSized<T, R<'b> = DynLockReadGuard<'b, T>, W<'b> = DynLockWriteGuard<'b, T>>>;

// pub type LockReadGuardBox<'a, T> = Box<dyn LockReadGuardlike<'a, T, R = LockReadGuardBox<'a, T>, W = LockWriteGuardBox<'a, T>>>;
//
// pub type LockWriteGuardBox<'a, T> = Box<dyn LockWriteGuardlike<'a, T, R = Box<dyn LockReadGuardlike<'a>>>>;

pub type DynLockUpgradeOutcome<'a, T> =
UpgradeOutcome<DynLockWriteGuard<'a, T>, DynLockReadGuard<'a, T>>;


pub trait Locklike<T: ?Sized>: Sync + Send {
    type R<'b> where
        Self: 'b;

    type W<'b> where
        Self: 'b;

    fn read<'b>(&'b self) -> Self::R<'b>;

    fn try_read<'b>(&'b self, duration: Duration) -> Option<Self::R<'b>>;

    fn write<'b>(&'b self) -> Self::W<'b>;

    fn try_write<'b>(&'b self, duration: Duration) -> Option<Self::W<'b>>;

    fn downgrade<'b>(&'b self, guard: Self::W<'b>) -> Self::R<'b>;

    fn upgrade<'b>(&'b self, guard: Self::R<'b>) -> Self::W<'b>;

    fn try_upgrade<'b>(&'b self, guard: Self::R<'b>, duration: Duration) -> UpgradeOutcome<Self::W<'b>, Self::R<'b>>;

    fn get_mut(&mut self) -> &mut T;
}

pub trait LocklikeSized<T>: Locklike<T> {
    fn into_inner(self: Box<Self>) -> T;
}

trait LockReadGuardlike<'a, T: ?Sized>: Deref<Target = T> {
    fn upgrade_box(self: Box<Self>) -> DynLockWriteGuard<'a, T>;

    fn try_upgrade_box(self: Box<Self>, duration: Duration) -> DynLockUpgradeOutcome<'a, T>;
}

impl<'a, T: ?Sized, M: Moderator> LockReadGuardlike<'a, T> for LockReadGuard<'a, T, M> {
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

impl<'a, T: ?Sized, M: Moderator> From<LockReadGuard<'a, T, M>> for DynLockReadGuard<'a, T> {
    fn from(guard: LockReadGuard<'a, T, M>) -> Self {
        DynLockReadGuard(Box::new(guard))
    }
}

trait LockWriteGuardlike<'a, T: ?Sized>: DerefMut<Target = T> {
    fn downgrade_box(self: Box<Self>) -> DynLockReadGuard<'a, T>;
}

impl<'a, T: ?Sized, M: Moderator> LockWriteGuardlike<'a, T> for LockWriteGuard<'a, T, M> {
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

impl<'a, T: ?Sized, M: Moderator> From<LockWriteGuard<'a, T, M>> for DynLockWriteGuard<'a, T> {
    fn from(guard: LockWriteGuard<'a, T, M>) -> Self {
        DynLockWriteGuard(Box::new(guard))
    }
}

// impl<T: ?Sized + Sync + Send, M: Moderator> Locklike<T> for XLock<T, M> {
//     type R<'b> = LockReadGuard<'b, T, M>;
//     type W<'b> = LockWriteGuard<'b, T, M>;
//
//     fn read<'b>(&self) -> Self::R<'b> {
//         self.read()
//     }
//
//     fn try_read<'b>(&'b self, duration: Duration) -> Option<Self::R<'b>> {
//         self.try_read(duration)
//     }
//
//     fn write<'b>(&'b self) -> Self::W<'b> {
//         self.write()
//     }
//
//     fn try_write<'b>(&'b self, duration: Duration) -> Option<Self::W<'b>> {
//         self.try_write(duration)
//     }
//
//     fn downgrade<'b>(&self, guard: Self::W<'b>) -> Self::R<'b> {
//         guard.downgrade()
//     }
//
//     fn upgrade<'b>(&self, guard: Self::R<'b>) -> Self::W<'b> {
//         guard.upgrade()
//     }
//
//     fn try_upgrade<'b>(&self, guard: Self::R<'b>, duration: Duration) -> UpgradeOutcome<Self::W<'b>, Self::R<'b>> {
//         guard.try_upgrade(duration)
//     }
//
//     fn get_mut(&mut self) -> &mut T {
//         self.get_mut()
//     }
// }

pub struct PolyLock<T: ?Sized, M: Moderator>(pub XLock<T, M>);

// pub struct PolyLockReadGuard<'a, T: ?Sized, M: Moderator>(LockReadGuard<'a, T, M>);
//
// pub struct PolyLockWriteGuard<'a, T: ?Sized, M: Moderator>(LockWriteGuard<'a, T, M>);

// impl<'a, T: ?Sized, M: Moderator> From<LockReadGuard<'a, T, M>> for PolyLockReadGuard<'a, T, M> {
//     fn from(guard: LockReadGuard<'a, T, M>) -> Self {
//         PolyLockReadGuard(guard)
//     }
// }
//
// impl<'a, T: ?Sized, M: Moderator> Deref for PolyLockReadGuard<'a, T, M> {
//     type Target = T;
//
//     fn deref(&self) -> &Self::Target {
//         self.0.deref()
//     }
// }
//
// impl<'a, T: ?Sized, M: Moderator> LockReadGuardlike<'a, T> for PolyLockReadGuard<'a, T, M> {
//     type R = PolyLockReadGuard<'a, T, M>;
//     type W = PolyLockWriteGuard<'a, T, M>;
//
//     fn upgrade(self) -> Self::W {
//         self.0.upgrade().into()
//     }
//
//     fn try_upgrade(self, duration: Duration) -> UpgradeOutcome<Self::W, Self::R> {
//         self.0.try_upgrade(duration).map(PolyLockWriteGuard::from, PolyLockReadGuard::from)
//     }
// }
//
// impl<'a, T: ?Sized, M: Moderator> From<LockWriteGuard<'a, T, M>> for PolyLockWriteGuard<'a, T, M> {
//     fn from(guard: LockWriteGuard<'a, T, M>) -> Self {
//         PolyLockWriteGuard(guard)
//     }
// }
//
// impl<'a, T: ?Sized, M: Moderator> Deref for PolyLockWriteGuard<'a, T, M> {
//     type Target = T;
//
//     fn deref(&self) -> &Self::Target {
//         self.0.deref()
//     }
// }
//
// impl<'a, T: ?Sized, M: Moderator> DerefMut for PolyLockWriteGuard<'a, T, M> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         self.0.deref_mut()
//     }
// }

// impl<'a, T: ?Sized, M: Moderator> LockWriteGuardlike<'a, T> for PolyLockWriteGuard<'a, T, M> {
//     type R = PolyLockReadGuard<'a, T, M>;
//
//     fn downgrade(self) -> Self::R {
//         self.0.downgrade().into()
//     }
// }

impl<T: ?Sized + Sync + Send, M: Moderator + Sync + Send> Locklike<T> for PolyLock<T, M> {
    type R<'b> = DynLockReadGuard<'b, T>;
    type W<'b> = DynLockWriteGuard<'b, T>;

    fn read<'b>(&'b self) -> Self::R<'b> {
        self.0.read().into()
    }

    fn try_read<'b>(&'b self, duration: Duration) -> Option<Self::R<'b>> {
        self.0.try_read(duration).map(DynLockReadGuard::from)
    }

    fn write<'b>(&'b self) -> Self::W<'b> {
        self.0.write().into()
    }

    fn try_write<'b>(&'b self, duration: Duration) -> Option<Self::W<'b>> {
        self.0.try_write(duration).map(DynLockWriteGuard::from)
    }

    fn downgrade<'b>(&'b self, guard: Self::W<'b>) -> Self::R<'b> {
        guard.downgrade()
    }

    fn upgrade<'b>(&'b self, guard: Self::R<'b>) -> Self::W<'b> {
        guard.upgrade()
    }

    fn try_upgrade<'b>(&'b self, guard: Self::R<'b>, duration: Duration) -> UpgradeOutcome<Self::W<'b>, Self::R<'b>> {
        guard.try_upgrade(duration)
    }

    fn get_mut(&mut self) -> &mut T {
        self.0.get_mut()
    }
}

impl<'a, T: Sync + Send + 'a, M: Moderator + Sync + Send +'a> LocklikeSized<T> for PolyLock<T, M> {
    fn into_inner(self: Box<Self>) -> T {
        self.0.into_inner()
    }
}

impl<T, M: Moderator> XLock<T, M> {
    fn lock_into_inner(self) -> T {
        self.into_inner()
    }
}

// impl<'a, T: Sync + Send + 'a, M: Moderator + 'a> LocklikeSized<T> for XLock<T, M> {
//     fn into_inner(self: Box<Self>) -> T {
//         self.lock_into_inner()
//     }
// }

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
    pub fn make_lock_for_test<'a, T: Sync + Send + 'static>(&self, t: T) -> LockBoxSized<'_, T> {
        println!("test running with moderator {:?}", self);
        match self {
            ModeratorKind::ReadBiased => Box::new(PolyLock(XLock::<_, ReadBiased>::new(t))),
            ModeratorKind::WriteBiased => Box::new(PolyLock(XLock::<_, WriteBiased>::new(t))),
            ModeratorKind::ArrivalOrdered => Box::new(PolyLock(XLock::<_, ArrivalOrdered>::new(t))),
        }
    }
}