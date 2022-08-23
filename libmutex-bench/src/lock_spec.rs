use std::ops::{Deref, DerefMut};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;
use libmutex::utils;
use libmutex::xlock::{LockReadGuard, LockWriteGuard, Moderator, UpgradeOutcome, XLock};

pub trait ReadGuardSpec<'a, T>: Deref<Target = T> {}

pub trait WriteGuardSpec<'a, T>: DerefMut<Target = T> {}

pub trait LockSpec<'a>: Sync + Send {
    type T: 'a;
    type R: ReadGuardSpec<'a, Self::T>;
    type W: WriteGuardSpec<'a, Self::T>;

    fn new(t: Self::T) -> Self;

    fn supports_downgrade() -> bool;

    fn supports_upgrade() -> bool;

    fn try_read(&'a self, duration: Duration) -> Option<Self::R>;

    fn try_write(&'a self, duration: Duration) -> Option<Self::W>;

    fn downgrade(guard: Self::W) -> Self::R;

    fn try_upgrade(guard: Self::R, duration: Duration) -> UpgradeOutcome<Self::W, Self::R>;
}

impl<'a, T, M: Moderator> ReadGuardSpec<'a, T> for LockReadGuard<'a, T, M> {}

impl<'a, T, M: Moderator> WriteGuardSpec<'a, T> for LockWriteGuard<'a, T, M> {}

impl<'a, T: Sync + Send + 'a, M: Moderator + 'a> LockSpec<'a> for XLock<T, M> {
    type T = T;
    type R = LockReadGuard<'a, T, M>;
    type W = LockWriteGuard<'a, T, M>;

    fn new(t: Self::T) -> Self {
        XLock::<_, M>::new(t)
    }

    fn supports_downgrade() -> bool {
        true
    }

    fn supports_upgrade() -> bool {
        true
    }

    #[inline]
    fn try_read(&'a self, duration: Duration) -> Option<Self::R> {
        self.try_read(duration)
    }

    #[inline]
    fn try_write(&'a self, duration: Duration) -> Option<Self::W> {
        self.try_write(duration)
    }

    #[inline]
    fn downgrade(guard: Self::W) -> Self::R {
        guard.downgrade()
    }

    #[inline]
    fn try_upgrade(guard: Self::R, duration: Duration) -> UpgradeOutcome<Self::W, Self::R> {
        guard.try_upgrade(duration)
    }
}

impl<'a, T> ReadGuardSpec<'a, T> for RwLockReadGuard<'a, T> {}

impl<'a, T> WriteGuardSpec<'a, T> for RwLockWriteGuard<'a, T> {}

impl<'a, T: Sync + Send + 'a> LockSpec<'a> for RwLock<T> {
    type T = T;
    type R = RwLockReadGuard<'a, T>;
    type W = RwLockWriteGuard<'a, T>;

    fn new(t: Self::T) -> Self {
        RwLock::new(t)
    }

    fn supports_downgrade() -> bool {
        false
    }

    fn supports_upgrade() -> bool {
        false
    }

    fn try_read(&'a self, duration: Duration) -> Option<Self::R> {
        if duration == Duration::MAX {
            Some(utils::remedy(self.read()))
        } else {
            utils::try_remedy(self.try_read())
        }
    }

    fn try_write(&'a self, duration: Duration) -> Option<Self::W> {
        if duration == Duration::MAX {
            Some(utils::remedy(self.write()))
        } else {
            utils::try_remedy(self.try_write())
        }
    }

    fn downgrade(_guard: Self::W) -> Self::R {
        unimplemented!()
    }

    fn try_upgrade(_guard: Self::R, _duration: Duration) -> UpgradeOutcome<Self::W, Self::R> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests;