use std::cell::UnsafeCell;
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::time::Duration;

mod read_biased;
mod write_biased;
mod arrival_ordered;
mod stochastic;
mod legacy_read_biased;
mod legacy_write_biased;
mod legacy_arrival_ordered;

pub use read_biased::ReadBiased;
pub use write_biased::WriteBiased;
pub use arrival_ordered::ArrivalOrdered;
pub use stochastic::Stochastic;
pub use legacy_read_biased::LegacyReadBiased;
pub use legacy_write_biased::LegacyWriteBiased;
pub use legacy_arrival_ordered::LegacyArrivalOrdered;

unsafe impl<T: ?Sized + Send, M: Moderator> Send for ZLock<T, M> {}
unsafe impl<T: ?Sized + Send + Sync, M: Moderator> Sync for ZLock<T, M> {}
unsafe impl<T: ?Sized + Sync, M: Moderator> Sync for LockReadGuard<'_, T, M> {}
unsafe impl<T: ?Sized + Sync, M: Moderator> Sync for LockWriteGuard<'_, T, M> {}

pub trait Moderator: Debug {
    type Sync;

    fn new() -> Self::Sync;

    fn try_read(sync: &Self::Sync, duration: Duration) -> bool;

    fn read_unlock(sync: &Self::Sync);

    fn try_write(sync: &Self::Sync, duration: Duration) -> bool;

    fn write_unlock(sync: &Self::Sync);

    fn downgrade(sync: &Self::Sync);

    fn try_upgrade(sync: &Self::Sync, duration: Duration) -> bool;
}

pub struct ZLock<T: ?Sized, M: Moderator> {
    sync: M::Sync,
    data: UnsafeCell<T>,
}

impl<T, M: Moderator> ZLock<T, M> {
    #[inline]
    pub fn new(t: T) -> Self {
        Self {
            sync: M::new(),
            data: UnsafeCell::new(t),
        }
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized, M: Moderator> ZLock<T, M> {
    #[inline]
    pub fn read(&self) -> LockReadGuard<'_, T, M> {
        self.try_read(Duration::MAX).unwrap()
    }

    #[inline]
    pub fn try_read(&self, duration: Duration) -> Option<LockReadGuard<'_, T, M>> {
        if M::try_read(&self.sync, duration) {
            let data = unsafe { NonNull::new_unchecked(self.data.get()) };
            Some(LockReadGuard {
                data,
                lock: self,
                locked: true,
                __no_send: PhantomData::default(),
            })
        } else {
            None
        }
    }

    #[inline]
    fn read_unlock(&self) {
        M::read_unlock(&self.sync);
    }

    #[inline]
    pub fn write(&self) -> LockWriteGuard<'_, T, M> {
        self.try_write(Duration::MAX).unwrap()
    }

    #[inline]
    pub fn try_write(&self, duration: Duration) -> Option<LockWriteGuard<'_, T, M>> {
        if M::try_write(&self.sync, duration) {
            Some(LockWriteGuard {
                lock: self,
                locked: true,
                __no_send: PhantomData::default(),
            })
        } else {
            None
        }
    }

    #[inline]
    fn write_unlock(&self) {
        M::write_unlock(&self.sync);
    }

    #[inline]
    pub fn downgrade(&self) -> LockReadGuard<T, M> {
        M::downgrade(&self.sync);
        let data = unsafe { NonNull::new_unchecked(self.data.get()) };
        LockReadGuard {
            data,
            lock: self,
            locked: true,
            __no_send: PhantomData::default(),
        }
    }

    #[inline]
    fn upgrade(&self) -> LockWriteGuard<'_, T, M> {
        self.try_upgrade(Duration::MAX).unwrap()
    }

    #[inline]
    fn try_upgrade(&self, duration: Duration) -> Option<LockWriteGuard<'_, T, M>> {
        if M::try_upgrade(&self.sync, duration) {
            Some(LockWriteGuard {
                lock: self,
                locked: true,
                __no_send: PhantomData::default(),
            })
        } else {
            None
        }
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the [`MultiLock`] mutably, no actual locking needs to
    /// take place---the mutable borrow statically guarantees no locks exist.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

pub struct LockReadGuard<'a, T: ?Sized + 'a, M: Moderator + 'a> {
    data: NonNull<T>,
    lock: &'a ZLock<T, M>,
    locked: bool,

    /// Emulates !Send for the struct. (Until issue 68318 -- negative trait bounds -- is resolved.)
    __no_send: PhantomData<*const ()>,
}

impl<T: ?Sized, M: Moderator> Drop for LockReadGuard<'_, T, M> {
    #[inline]
    fn drop(&mut self) {
        if self.locked {
            self.lock.read_unlock();
        }
    }
}

impl<'a, T: ?Sized, M: Moderator> LockReadGuard<'a, T, M> {
    #[inline]
    pub fn upgrade(mut self) -> LockWriteGuard<'a, T, M> {
        self.locked = false;
        self.lock.upgrade()
    }

    #[inline]
    pub fn try_upgrade(mut self, duration: Duration) -> LockUpgradeOutcome<'a, T, M> {
        match self.lock.try_upgrade(duration) {
            None => UpgradeOutcome::Unchanged(self),
            Some(guard) => {
                self.locked = false;
                UpgradeOutcome::Upgraded(guard)
            }
        }
    }
}

impl<T: ?Sized, M: Moderator> Deref for LockReadGuard<'_, T, M> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

pub struct LockWriteGuard<'a, T: ?Sized + 'a, M: Moderator + 'a> {
    lock: &'a ZLock<T, M>,
    locked: bool,
    /// Emulates !Send for the struct. (Until issue 68318 -- negative trait bounds -- is resolved.)
    __no_send: PhantomData<*const ()>,
}

impl<T: ?Sized, M: Moderator> Drop for LockWriteGuard<'_, T, M> {
    #[inline]
    fn drop(&mut self) {
        if self.locked {
            self.lock.write_unlock();
        }
    }
}

impl<'a, T: ?Sized, M: Moderator> LockWriteGuard<'a, T, M> {
    #[inline]
    pub fn downgrade(mut self) -> LockReadGuard<'a, T, M> {
        self.locked = false;
        self.lock.downgrade()
    }
}

impl<T: ?Sized, M: Moderator> Deref for LockWriteGuard<'_, T, M> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized, M: Moderator> DerefMut for LockWriteGuard<'_, T, M> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

pub type LockUpgradeOutcome<'a, T, M> = UpgradeOutcome<LockWriteGuard<'a, T, M>, LockReadGuard<'a, T, M>>;

pub enum UpgradeOutcome<W, R> {
    Upgraded(W),
    Unchanged(R),
}

impl<W, R> UpgradeOutcome<W, R> {
    #[inline]
    pub fn is_upgraded(&self) -> bool {
        matches!(self, UpgradeOutcome::Upgraded(_))
    }

    #[inline]
    pub fn is_unchanged(&self) -> bool {
        matches!(self, UpgradeOutcome::Unchanged(_))
    }

    #[inline]
    pub fn upgraded(self) -> Option<W> {
        match self {
            UpgradeOutcome::Upgraded(guard) => Some(guard),
            UpgradeOutcome::Unchanged(_) => None,
        }
    }

    #[inline]
    pub fn unchanged(self) -> Option<R> {
        match self {
            UpgradeOutcome::Upgraded(_) => None,
            UpgradeOutcome::Unchanged(guard) => Some(guard),
        }
    }

    #[inline]
    pub fn map<WW, RR>(self, f_w: impl FnOnce(W) -> WW, f_r: impl FnOnce(R) -> RR) -> UpgradeOutcome<WW, RR> {
        match self {
            UpgradeOutcome::Upgraded(w) => UpgradeOutcome::Upgraded(f_w(w)),
            UpgradeOutcome::Unchanged(r) => UpgradeOutcome::Unchanged(f_r(r)),
        }
    }
}

impl<T: ?Sized + fmt::Debug, M: Moderator> fmt::Debug for ZLock<T, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("ZLock");
        match self.try_read(Duration::ZERO) {
            None => {
                struct LockedPlaceholder;
                impl fmt::Debug for LockedPlaceholder {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str("<locked>")
                    }
                }
                d.field("data", &LockedPlaceholder);
            }
            Some(guard) => {
                d.field("data", &&*guard);
            }
        }
        d.finish_non_exhaustive()
    }
}

#[cfg(test)]
mod faulty;

#[cfg(test)]
pub use faulty::Faulty;

#[cfg(test)]
pub mod locklike;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tr_tests;

#[cfg(test)]
mod std_tests;