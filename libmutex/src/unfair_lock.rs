use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::{
    Condvar, Mutex,
};
use std::time::{Duration};
use crate::deadline::Deadline;
use crate::utils;

#[derive(Debug, Default)]
struct InternalState {
    readers: u32,
    writer: bool,
}

#[derive(Debug)]
pub struct UnfairLock<T: ?Sized> {
    state: Mutex<InternalState>,
    cond: Condvar,
    data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for UnfairLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for UnfairLock<T> {}
unsafe impl<T: ?Sized + Sync> Sync for LockReadGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for LockWriteGuard<'_, T> {}

pub struct LockReadGuard<'a, T: ?Sized> {
    data: NonNull<T>,
    lock: &'a UnfairLock<T>,
    locked: bool,

    /// Emulates !Send for the struct. (Until issue 68318 -- negative trait bounds -- is resolved.)
    __no_send: PhantomData<*const ()>
}

impl<T: ?Sized> Drop for LockReadGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        if self.locked {
            self.lock.read_unlock();
        }
    }
}

impl<'a, T: ?Sized> LockReadGuard<'a, T> {
    #[inline]
    pub fn upgrade(mut self) -> LockWriteGuard<'a, T> {
        self.locked = false;
        self.lock.upgrade()
    }

    #[inline]
    pub fn try_upgrade(mut self, duration: Duration) -> UpgradeOutcome<'a, T> {
        match self.lock.try_upgrade(duration) {
            None => UpgradeOutcome::Unchanged(self),
            Some(guard) => {
                self.locked = false;
                UpgradeOutcome::Upgraded(guard)
            }
        }
    }
}

pub enum UpgradeOutcome<'a, T: ?Sized> {
    Upgraded(LockWriteGuard<'a, T>),
    Unchanged(LockReadGuard<'a, T>)
}

impl<'a, T: ?Sized> UpgradeOutcome<'a, T> {
    #[inline]
    pub fn is_upgraded(&self) -> bool {
        matches!(self, UpgradeOutcome::Upgraded(_))
    }

    #[inline]
    pub fn is_unchanged(&self) -> bool {
        matches!(self, UpgradeOutcome::Unchanged(_))
    }

    #[inline]
    pub fn upgraded(self) -> Option<LockWriteGuard<'a, T>> {
        match self {
            UpgradeOutcome::Upgraded(guard) => Some(guard),
            UpgradeOutcome::Unchanged(_) => None
        }
    }

    #[inline]
    pub fn unchanged(self) -> Option<LockReadGuard<'a, T>> {
        match self {
            UpgradeOutcome::Upgraded(_) => None,
            UpgradeOutcome::Unchanged(guard) => Some(guard)
        }
    }
}

impl<T: ?Sized> Deref for LockReadGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { self.data.as_ref() }
    }
}

pub struct LockWriteGuard<'a, T: ?Sized> {
    lock: &'a UnfairLock<T>,
    locked: bool,
    /// Emulates !Send for the struct. (Until issue 68318 -- negative trait bounds -- is resolved.)
    __no_send: PhantomData<*const ()>
}

impl<T: ?Sized> Drop for LockWriteGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        if self.locked {
            self.lock.write_unlock();
        }
    }
}

impl<'a, T: ?Sized> LockWriteGuard<'a, T> {
    #[inline]
    pub fn downgrade(mut self) -> LockReadGuard<'a, T> {
        self.locked = false;
        self.lock.downgrade()
    }
}

impl<T: ?Sized> Deref for LockWriteGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: ?Sized> DerefMut for LockWriteGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> UnfairLock<T> {
    #[inline]
    pub fn new(t: T) -> Self {
        Self {
            state: Mutex::new(InternalState::default()),
            cond: Condvar::new(),
            data: UnsafeCell::new(t),
        }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: ?Sized> UnfairLock<T> {
    #[inline]
    pub fn read(&self) -> LockReadGuard<'_, T> {
        self.try_read(Duration::MAX).unwrap()
    }

    #[inline]
    pub fn try_read(&self, duration: Duration) -> Option<LockReadGuard<'_, T>> {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = utils::remedy(self.state.lock());
        while state.writer {
            let (guard, timed_out) =
                utils::cond_wait_remedy(&self.cond, state, deadline.remaining());

            if timed_out {
                return None;
            }
            state = guard;
        }
        state.readers += 1;
        drop(state);

        let data = unsafe {
            NonNull::new_unchecked(self.data.get())
        };
        Some(LockReadGuard {
            data,
            lock: self,
            locked: true,
            __no_send: PhantomData::default()
        })
    }

    #[inline]
    fn read_unlock(&self) {
        let mut state = utils::remedy(self.state.lock());
        debug_assert!(state.readers > 0, "readers: {}", state.readers);
        debug_assert!(!state.writer);
        state.readers -= 1;
        let readers = state.readers;
        drop(state);
        if readers == 1 {
            self.cond.notify_all();
        } else if readers == 0 {
            self.cond.notify_one();
        }
    }

    #[inline]
    pub fn write(&self) -> LockWriteGuard<'_, T> {
        self.try_write(Duration::MAX).unwrap()
    }

    #[inline]
    pub fn try_write(
        &self,
        duration: Duration,
    ) -> Option<LockWriteGuard<'_, T>> {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = utils::remedy(self.state.lock());
        while state.readers != 0 || state.writer {
            let (guard, timed_out) =
                utils::cond_wait_remedy(&self.cond, state, deadline.remaining());

            if timed_out {
                return None;
            }
            state = guard;
        }
        state.writer = true;
        drop(state);

        Some(LockWriteGuard {
            lock: self,
            locked: true,
            __no_send: PhantomData::default()
        })
    }

    #[inline]
    fn write_unlock(&self) {
        let mut state = utils::remedy(self.state.lock());
        debug_assert!(state.readers == 0, "readers: {}", state.readers);
        debug_assert!(state.writer);
        state.writer = false;
        drop(state);
        self.cond.notify_one();
    }

    #[inline]
    fn downgrade(&self) -> LockReadGuard<'_, T> {
        let mut state = utils::remedy(self.state.lock());
        debug_assert!(state.readers == 0, "readers: {}", state.readers);
        debug_assert!(state.writer);
        state.readers = 1;
        state.writer = false;
        drop(state);
        self.cond.notify_all();
        let data = unsafe {
            NonNull::new_unchecked(self.data.get())
        };
        LockReadGuard {
            data,
            lock: self,
            locked: true,
            __no_send: PhantomData::default()
        }
    }

    #[inline]
    fn upgrade(&self) -> LockWriteGuard<'_, T> {
        self.try_upgrade(Duration::MAX).unwrap()
    }

    #[inline]
    fn try_upgrade(&self, duration: Duration) -> Option<LockWriteGuard<'_, T>> {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = utils::remedy(self.state.lock());
        debug_assert!(state.readers > 0, "readers: {}", state.readers);
        debug_assert!(!state.writer);
        while state.readers != 1 {
            let (guard, timed_out) =
                utils::cond_wait_remedy(&self.cond, state, deadline.remaining());

            if timed_out {
                return None;
            }
            state = guard;
            debug_assert!(state.readers > 0, "readers: {}", state.readers);
            debug_assert!(!state.writer);
        }
        state.readers = 0;
        state.writer = true;
        drop(state);
        Some(LockWriteGuard {
            lock: self,
            locked: true,
            __no_send: PhantomData::default()
        })
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the [`UnfairLock`] mutably, no actual locking needs to
    /// take place---the mutable borrow statically guarantees no locks exist.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

#[cfg(test)]
mod tr_tests;

#[cfg(test)]
mod pl_tests;