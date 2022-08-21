use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::time::Duration;

mod read_biased;
mod faulty;

pub use read_biased::ReadBiased;
pub use faulty::Faulty;

unsafe impl<T: ?Sized + Send, S: Spec> Send for XLock<T, S> {}
unsafe impl<T: ?Sized + Send + Sync, S: Spec> Sync for XLock<T, S> {}
unsafe impl<T: ?Sized + Sync, S: Spec> Sync for LockReadGuard<'_, T, S> {}
unsafe impl<T: ?Sized + Sync, S: Spec> Sync for LockWriteGuard<'_, T, S> {}

pub trait Spec: Debug {
    type Sync;

    fn new() -> Self::Sync;

    fn try_read(sync: &Self::Sync, duration: Duration) -> bool;

    fn read_unlock(sync: &Self::Sync);

    fn try_write(sync: &Self::Sync, duration: Duration) -> bool;

    fn write_unlock(sync: &Self::Sync);

    fn downgrade(sync: &Self::Sync);

    fn try_upgrade(sync: &Self::Sync, duration: Duration) -> bool;
}

#[derive(Debug)]
pub struct XLock<T: ?Sized, S: Spec> {
    sync: S::Sync,
    data: UnsafeCell<T>,
}

impl<T, S: Spec> XLock<T, S> {
    #[inline]
    pub fn new(t: T) -> Self {
        Self {
            sync: S::new(),
            data: UnsafeCell::new(t),
        }
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized, S: Spec> XLock<T, S> {
    #[inline]
    pub fn read(&self) -> LockReadGuard<'_, T, S> {
        self.try_read(Duration::MAX).unwrap()
    }

    #[inline]
    pub fn try_read(&self, duration: Duration) -> Option<LockReadGuard<'_, T, S>> {
        if S::try_read(&self.sync, duration) {
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
        S::read_unlock(&self.sync);
    }

    #[inline]
    pub fn write(&self) -> LockWriteGuard<'_, T, S> {
        self.try_write(Duration::MAX).unwrap()
    }

    #[inline]
    pub fn try_write(&self, duration: Duration) -> Option<LockWriteGuard<'_, T, S>> {
        if S::try_write(&self.sync, duration) {
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
        S::write_unlock(&self.sync);
    }

    #[inline]
    pub fn downgrade(&self) -> LockReadGuard<T, S> {
        S::downgrade(&self.sync);
        let data = unsafe { NonNull::new_unchecked(self.data.get()) };
        LockReadGuard {
            data,
            lock: self,
            locked: true,
            __no_send: PhantomData::default(),
        }
    }

    #[inline]
    fn upgrade(&self) -> LockWriteGuard<'_, T, S> {
        self.try_upgrade(Duration::MAX).unwrap()
    }

    #[inline]
    fn try_upgrade(&self, duration: Duration) -> Option<LockWriteGuard<'_, T, S>> {
        if S::try_upgrade(&self.sync, duration) {
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

pub struct LockReadGuard<'a, T: ?Sized, S: Spec> {
    data: NonNull<T>,
    lock: &'a XLock<T, S>,
    locked: bool,

    /// Emulates !Send for the struct. (Until issue 68318 -- negative trait bounds -- is resolved.)
    __no_send: PhantomData<*const ()>,
}

impl<T: ?Sized, S: Spec> Drop for LockReadGuard<'_, T, S> {
    #[inline]
    fn drop(&mut self) {
        if self.locked {
            self.lock.read_unlock();
        }
    }
}

impl<'a, T: ?Sized, S: Spec> LockReadGuard<'a, T, S> {
    #[inline]
    pub fn upgrade(mut self) -> LockWriteGuard<'a, T, S> {
        self.locked = false;
        self.lock.upgrade()
    }

    #[inline]
    pub fn try_upgrade(mut self, duration: Duration) -> LockUpgradeOutcome<'a, T, S> {
        match self.lock.try_upgrade(duration) {
            None => UpgradeOutcome::Unchanged(self),
            Some(guard) => {
                self.locked = false;
                UpgradeOutcome::Upgraded(guard)
            }
        }
    }
}

impl<T: ?Sized, S: Spec> Deref for LockReadGuard<'_, T, S> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

pub struct LockWriteGuard<'a, T: ?Sized, S: Spec> {
    lock: &'a XLock<T, S>,
    locked: bool,
    /// Emulates !Send for the struct. (Until issue 68318 -- negative trait bounds -- is resolved.)
    __no_send: PhantomData<*const ()>,
}

impl<T: ?Sized, S: Spec> Drop for LockWriteGuard<'_, T, S> {
    #[inline]
    fn drop(&mut self) {
        if self.locked {
            self.lock.write_unlock();
        }
    }
}

impl<'a, T: ?Sized, S: Spec> LockWriteGuard<'a, T, S> {
    #[inline]
    pub fn downgrade(mut self) -> LockReadGuard<'a, T, S> {
        self.locked = false;
        self.lock.downgrade()
    }
}

impl<T: ?Sized, S: Spec> Deref for LockWriteGuard<'_, T, S> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized, S: Spec> DerefMut for LockWriteGuard<'_, T, S> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

pub type LockUpgradeOutcome<'a, T, S> = UpgradeOutcome<LockWriteGuard<'a, T, S>, LockReadGuard<'a, T, S>>;

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

#[cfg(test)]
mod tests;

#[cfg(test)]
pub mod locklike;