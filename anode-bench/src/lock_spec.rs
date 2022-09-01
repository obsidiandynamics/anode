use anode::spin_mutex::{SpinGuard, SpinMutex};
use anode::remedy::Remedy;
use anode::zlock::{LockReadGuard, LockWriteGuard, Moderator, UpgradeOutcome, ZLock};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::{MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;

pub trait ReadGuardSpec<'a, T>: Deref<Target = T> {}

pub trait WriteGuardSpec<'a, T>: DerefMut<Target = T> {}

pub trait LockSpec<'a>: Sync + Send {
    type T: 'a;
    type R: ReadGuardSpec<'a, Self::T>;
    type W: WriteGuardSpec<'a, Self::T>;

    fn new(t: Self::T) -> Self;

    fn supports_read() -> bool;

    fn supports_downgrade() -> bool;

    fn supports_upgrade() -> bool;

    fn try_read(&'a self, duration: Duration) -> Option<Self::R>;

    fn try_write(&'a self, duration: Duration) -> Option<Self::W>;

    fn downgrade(guard: Self::W) -> Self::R;

    fn try_upgrade(guard: Self::R, duration: Duration) -> UpgradeOutcome<Self::W, Self::R>;
}

pub struct NoReadGuard<T> {
    __phantom_data: PhantomData<T>,
}

impl<T> Default for NoReadGuard<T> {
    fn default() -> Self {
        Self {
            __phantom_data: PhantomData::default(),
        }
    }
}

impl<T> Deref for NoReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unimplemented!()
    }
}

impl<'a, T> ReadGuardSpec<'a, T> for NoReadGuard<T> {}

impl<'a, T, M: Moderator> ReadGuardSpec<'a, T> for LockReadGuard<'a, T, M> {}

impl<'a, T, M: Moderator> WriteGuardSpec<'a, T> for LockWriteGuard<'a, T, M> {}

impl<'a, T: Sync + Send + 'a, M: Moderator + 'a> LockSpec<'a> for ZLock<T, M> {
    type T = T;
    type R = LockReadGuard<'a, T, M>;
    type W = LockWriteGuard<'a, T, M>;

    fn new(t: Self::T) -> Self {
        ZLock::<_, M>::new(t)
    }

    fn supports_read() -> bool {
        true
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
        Self::new(t)
    }

    fn supports_read() -> bool {
        true
    }

    fn supports_downgrade() -> bool {
        false
    }

    fn supports_upgrade() -> bool {
        false
    }

    fn try_read(&'a self, duration: Duration) -> Option<Self::R> {
        if duration == Duration::MAX {
            Some(self.read().remedy())
        } else {
            self.try_read().remedy()
        }
    }

    fn try_write(&'a self, duration: Duration) -> Option<Self::W> {
        if duration == Duration::MAX {
            Some(self.write().remedy())
        } else {
            self.try_write().remedy()
        }
    }

    fn downgrade(_guard: Self::W) -> Self::R {
        unimplemented!()
    }

    fn try_upgrade(_guard: Self::R, _duration: Duration) -> UpgradeOutcome<Self::W, Self::R> {
        unimplemented!()
    }
}

impl<'a, T> WriteGuardSpec<'a, T> for SpinGuard<'a, T> {}

impl<'a, T: Sync + Send + 'a> LockSpec<'a> for SpinMutex<T> {
    type T = T;
    type R = NoReadGuard<T>;
    type W = SpinGuard<'a, T>;

    fn new(t: Self::T) -> Self {
        Self::new(t)
    }

    fn supports_read() -> bool {
        false
    }

    fn supports_downgrade() -> bool {
        false
    }

    fn supports_upgrade() -> bool {
        false
    }

    fn try_read(&'a self, _duration: Duration) -> Option<Self::R> {
        unimplemented!()
    }

    fn try_write(&'a self, duration: Duration) -> Option<Self::W> {
        if duration == Duration::MAX {
            Some(self.lock())
        } else {
            self.try_lock()
        }
    }

    fn downgrade(_guard: Self::W) -> Self::R {
        unimplemented!()
    }

    fn try_upgrade(_guard: Self::R, _duration: Duration) -> UpgradeOutcome<Self::W, Self::R> {
        unimplemented!()
    }
}

impl<'a, T> WriteGuardSpec<'a, T> for MutexGuard<'a, T> {}

impl<'a, T: Sync + Send + 'a> LockSpec<'a> for std::sync::Mutex<T> {
    type T = T;
    type R = NoReadGuard<T>;
    type W = MutexGuard<'a, T>;

    fn new(t: Self::T) -> Self {
        Self::new(t)
    }

    fn supports_read() -> bool {
        false
    }

    fn supports_downgrade() -> bool {
        false
    }

    fn supports_upgrade() -> bool {
        false
    }

    fn try_read(&'a self, _duration: Duration) -> Option<Self::R> {
        unimplemented!()
    }

    fn try_write(&'a self, duration: Duration) -> Option<Self::W> {
        if duration == Duration::MAX {
            Some(self.lock().remedy())
        } else {
            self.try_lock().remedy()
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
