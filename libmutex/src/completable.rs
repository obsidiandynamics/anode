use crate::deadline::Deadline;
use crate::utils;
use std::ops::{Deref};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Duration;

#[derive(Default, Debug)]
pub struct Completable<T> {
    cond: Condvar,
    data: Mutex<Option<T>>,
}

#[derive(Debug)]
pub struct Completed<'a, T> {
    guard: MutexGuard<'a, Option<T>>,
}

impl<T> Deref for Completed<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.as_ref().unwrap()
    }
}

impl<T> Completable<T> {
    #[inline]
    pub fn new(val: T) -> Self {
        Self {
            cond: Condvar::default(),
            data: Mutex::new(Some(val))
        }
    }

    #[inline]
    pub fn complete(&self, val: T) -> bool {
        let mut data = utils::remedy(self.data.lock());
        if data.is_none() {
            *data = Some(val);
            drop(data);
            self.cond.notify_all();
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn is_complete(&self) -> bool {
        utils::remedy(self.data.lock()).is_some()
    }

    #[inline]
    pub fn wait(&self) -> Completed<T> {
        Completed {
            guard: self.__try_wait(Duration::MAX),
        }
    }

    #[inline]
    pub fn get<'a>(&'a self) -> impl Deref<Target = Option<T>> + 'a {
        self.__try_wait(Duration::ZERO)
    }

    #[inline]
    pub fn try_wait<'a>(&'a self, duration: Duration) -> impl Deref<Target = Option<T>> + 'a {
        self.__try_wait(duration)
    }

    /// [`__try_wait`] is never exposed directly to avoid coupling the caller to the
    /// [`MutexGuard`] type, which might change in future implementations. Instead, the mutex guard is
    /// publicly exposed as a [`Deref`] trait.
    #[inline]
    fn __try_wait(&self, duration: Duration) -> MutexGuard<Option<T>> {
        let mut deadline = Deadline::lazy_after(duration);
        let mut data = utils::remedy(self.data.lock());
        while data.is_none() {
            let (guard, timed_out) =
                utils::cond_wait_remedy(&self.cond, data, deadline.remaining());
            if timed_out {
                return guard;
            }
            data = guard;
        }
        data
    }

    pub fn into_inner(self) -> Option<T> {
        utils::remedy(self.data.into_inner())
    }
}

#[cfg(test)]
mod tests;