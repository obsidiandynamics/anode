use crate::deadline::Deadline;
use crate::remedy;
use std::ops::{Deref};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Duration;
use crate::remedy::Remedy;

#[derive(Default, Debug)]
pub struct Completable<T> {
    cond: Condvar,
    data: Mutex<Option<T>>,
}

#[derive(Debug)]
pub struct Completed<'a, T> {
    guard: MutexGuard<'a, Option<T>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome<T> {
    Abort,
    Success(T)
}

impl<T> Outcome<T> {
    #[inline]
    pub fn is_abort(&self) -> bool {
        matches!(self, Outcome::Abort)
    }

    #[inline]
    pub fn is_success(&self) -> bool {
        matches!(self, Outcome::Success(_))
    }

    #[inline]
    pub fn into_option(self) -> Option<T> {
        match self {
            Outcome::Abort => None,
            Outcome::Success(val) => Some(val)
        }
    }
}

impl<T> Default for Outcome<T> {
    #[inline]
    fn default() -> Self {
        Outcome::Abort
    }
}

impl<T> Deref for Completed<'_, T> {
    type Target = T;

    #[inline]
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

    /// Completes this instance in exclusive mode, wherein the given closure is
    /// atomically invoked if and only if the instance is incomplete. No other
    /// thread may succeed in completing this instance in the meantime.
    ///
    /// This method is used when `f` represents an expensive computation, the result of
    /// which should not be discarded. Conversely, if this instance is already
    /// complete, `f` should not be attempted.
    ///
    /// Returns `true` if and only if `f` was invoked.
    #[inline]
    pub fn complete_exclusive(&self, f: impl FnOnce() -> T) -> bool {
        let mut data = self.data.lock().remedy();
        if data.is_none() {
            *data = Some(f());
            drop(data);
            self.cond.notify_all();
            true
        } else {
            false
        }
    }

    /// Completes this instance, assigning `val` if the instance is incomplete. Otherwise,
    /// the existing completed value is preserved.
    ///
    /// Returns `true` if and only if `val` was assigned.
    #[inline]
    pub fn complete(&self, val: T) -> bool {
        self.complete_exclusive(|| val)
    }

    #[inline]
    pub fn is_complete(&self) -> bool {
        self.data.lock().remedy().is_some()
    }

    #[inline]
    pub fn get(&self) -> Completed<T> {
        Completed {
            guard: self.__try_get(Duration::MAX),
        }
    }

    #[inline]
    pub fn peek<'a>(&'a self) -> impl Deref<Target = Option<T>> + 'a {
        self.__try_get(Duration::ZERO)
    }

    #[inline]
    pub fn try_get<'a>(&'a self, duration: Duration) -> impl Deref<Target = Option<T>> + 'a {
        self.__try_get(duration)
    }

    /// [`__try_get`] is never exposed directly to avoid coupling the caller to the
    /// [`MutexGuard`] type, which might change in future implementations. Instead, the return
    /// value is publicly exposed as a [`Deref`] trait.
    #[inline]
    fn __try_get(&self, duration: Duration) -> MutexGuard<Option<T>> {
        let mut deadline = Deadline::lazy_after(duration);
        let mut data = self.data.lock().remedy();
        while data.is_none() {
            let (guard, timed_out) =
                remedy::cond_wait_remedy(&self.cond, data, deadline.remaining());
            if timed_out {
                return guard;
            }
            data = guard;
        }
        data
    }

    pub fn into_inner(self) -> Option<T> {
        self.data.into_inner().remedy()
    }
}

#[cfg(test)]
mod tests;