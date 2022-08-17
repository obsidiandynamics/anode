use std::ops::{Deref, DerefMut};
use std::sync::{
    Condvar, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
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
    data: RwLock<T>,
}

pub struct LockReadGuard<'a, T: ?Sized> {
    data: Option<RwLockReadGuard<'a, T>>,
    lock: &'a UnfairLock<T>,
}

impl<T: ?Sized> Drop for LockReadGuard<'_, T> {
    fn drop(&mut self) {
        if let Some(_) = self.data.take() {
            self.lock.read_unlock();
        }
    }
}

impl<'a, T: ?Sized> LockReadGuard<'a, T> {
    pub fn upgrade(mut self) -> LockWriteGuard<'a, T> {
        self.data = None;
        self.lock.upgrade()
    }

    pub fn try_upgrade(mut self, duration: Duration) -> MaybeUpgraded<'a, T> {
        self.data = None;
        match self.lock.try_upgrade(duration) {
            None => {
                self.data = Some(self.lock.restore_read_guard());
                MaybeUpgraded::Unchanged(self)
            }
            Some(guard) => MaybeUpgraded::Upgraded(guard)
        }
    }
}

pub enum MaybeUpgraded<'a, T: ?Sized> {
    Upgraded(LockWriteGuard<'a, T>),
    Unchanged(LockReadGuard<'a, T>)
}

impl<'a, T: ?Sized> MaybeUpgraded<'a, T> {
    pub fn is_upgraded(&self) -> bool {
        matches!(self, MaybeUpgraded::Upgraded(_))
    }

    pub fn is_unchanged(&self) -> bool {
        matches!(self, MaybeUpgraded::Unchanged(_))
    }

    pub fn upgraded(self) -> Option<LockWriteGuard<'a, T>> {
        match self {
            MaybeUpgraded::Upgraded(guard) => Some(guard),
            MaybeUpgraded::Unchanged(_) => None
        }
    }

    pub fn unchanged(self) -> Option<LockReadGuard<'a, T>> {
        match self {
            MaybeUpgraded::Upgraded(_) => None,
            MaybeUpgraded::Unchanged(guard) => Some(guard)
        }
    }
}

impl<T: ?Sized> Deref for LockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data.as_ref().unwrap().deref()
    }
}

pub struct LockWriteGuard<'a, T: ?Sized> {
    data: Option<RwLockWriteGuard<'a, T>>,
    lock: &'a UnfairLock<T>,
}

impl<T: ?Sized> Drop for LockWriteGuard<'_, T> {
    fn drop(&mut self) {
        if let Some(_) = self.data.take() {
            self.lock.write_unlock();
        }
    }
}

impl<'a, T: ?Sized> LockWriteGuard<'a, T> {
    pub fn downgrade(mut self) -> LockReadGuard<'a, T> {
        self.data = None;
        self.lock.downgrade()
    }
}

impl<T: ?Sized> Deref for LockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data.as_ref().unwrap().deref()
    }
}

impl<T: ?Sized> DerefMut for LockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data.as_mut().unwrap().deref_mut()
    }
}

impl<T> UnfairLock<T> {
    pub fn new(t: T) -> Self {
        Self {
            state: Mutex::new(InternalState::default()),
            cond: Condvar::new(),
            data: RwLock::new(t),
        }
    }

    pub fn into_inner(self) -> T {
        let (data, _) = utils::unpack(self.data.into_inner());
        data
    }
}

impl<T: ?Sized> UnfairLock<T> {
    pub fn read(&self) -> LockReadGuard<'_, T> {
        self.try_read(Duration::MAX).unwrap()
    }

    pub fn try_read(&self, duration: Duration) -> Option<LockReadGuard<'_, T>> {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = self.state.lock().unwrap();
        while state.writer {
            let (guard, timed_out) =
                utils::cond_wait(&self.cond, state, deadline.remaining()).unwrap();

            if timed_out {
                return None;
            }
            state = guard;
        }
        state.readers += 1;
        drop(state);

        let (data, _) = utils::unpack(self.data.read());
        Some(LockReadGuard {
            data: Some(data),
            lock: self,
        })
    }

    fn read_unlock(&self) {
        let mut state = self.state.lock().unwrap();
        assert!(state.readers > 0, "readers: {}", state.readers);
        state.readers -= 1;
        let readers = state.readers;
        drop(state);
        if readers == 1 {
            self.cond.notify_all();
        } else if readers == 0 {
            self.cond.notify_one();
        }
    }

    pub fn write(&self) -> LockWriteGuard<'_, T> {
        self.try_write(Duration::MAX).unwrap()
    }

    pub fn try_write(
        &self,
        duration: Duration,
    ) -> Option<LockWriteGuard<'_, T>> {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = self.state.lock().unwrap();
        while state.readers != 0 || state.writer {
            // println!("Remaining {remaining:?}, duration={duration:?}, deadline={deadline:?}");
            let (guard, timed_out) =
                utils::cond_wait(&self.cond, state, deadline.remaining()).unwrap();

            if timed_out {
                return None;
            }
            state = guard;
        }
        state.writer = true;
        drop(state);

        let (data, _) = utils::unpack(self.data.write());
        Some(LockWriteGuard {
            data: Some(data),
            lock: self,
        })
    }

    fn write_unlock(&self) {
        let mut state = self.state.lock().unwrap();
        state.writer = false;
        drop(state);
        self.cond.notify_one();
    }

    fn downgrade(&self) -> LockReadGuard<'_, T> {
        let mut state = self.state.lock().unwrap();
        state.readers = 1;
        state.writer = false;
        drop(state);
        self.cond.notify_all();
        let (data, _) = utils::unpack(self.data.read());
        LockReadGuard {
            data: Some(data),
            lock: self,
        }
    }

    fn upgrade(&self) -> LockWriteGuard<'_, T> {
        self.try_upgrade(Duration::MAX).unwrap()
    }

    fn try_upgrade(&self, duration: Duration) -> Option<LockWriteGuard<'_, T>> {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = self.state.lock().unwrap();
        while state.readers != 1 {
            let (guard, timed_out) =
                utils::cond_wait(&self.cond, state, deadline.remaining()).unwrap();

            if timed_out {
                return None;
            }
            state = guard;
        }
        state.readers = 0;
        state.writer = true;
        drop(state);
        let (data, _) = utils::unpack(self.data.write());
        Some(LockWriteGuard {
            data: Some(data),
            lock: self,
        })
    }

    fn restore_read_guard(&self) -> RwLockReadGuard<'_, T> {
        let (data, _) = utils::unpack(self.data.read());
        data
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the [`UnfairLock`] mutably, no actual locking needs to
    /// take place---the mutable borrow statically guarantees no locks exist.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        let (data, _) = utils::unpack(self.data.get_mut());
        data
    }
}

#[cfg(test)]
mod transram_tests;

#[cfg(test)]
mod parking_lot_tests;