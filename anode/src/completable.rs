use crate::deadline::Deadline;
use std::ops::{Deref};
use std::time::Duration;
use crate::monitor::{Directive, Monitor, SpeculativeMonitor, SpeculativeMonitorGuard};

#[derive(Default, Debug)]
pub struct Completable<T> {
    monitor: SpeculativeMonitor<Option<T>>,
}

pub struct Completed<'a, T> {
    guard: SpeculativeMonitorGuard<'a, Option<T>>,
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
            monitor: SpeculativeMonitor::new(Some(val))
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
    pub fn complete_exclusive<F: FnOnce() -> T>(&self, f: F) -> bool {
        let mut f = Some(f);
        self.monitor.enter(|val| {
            if val.is_none() {
                *val = Some(f.take().unwrap()());
            }

            if f.is_none() {
                Directive::NotifyAll
            } else {
                Directive::Return
            }
        });
        f.is_none()
    }

    /// Completes this instance, assigning `val` if the instance is incomplete. Otherwise,
    /// the existing completed value is preserved and `val` is returned to the caller.
    ///
    /// Returns `None` if the given value was persisted or `Some` containing the value if
    /// it could not be assigned.
    #[inline]
    pub fn complete(&self, val: T) -> Option<T> {
        let mut returned = Some(val);
        self.monitor.enter(|inner| {
            if inner.is_none() {
                *inner = returned.take();
            }

            if returned.is_none() {
                Directive::NotifyAll
            } else {
                Directive::Return
            }
        });
        returned
    }

    #[inline]
    pub fn is_complete(&self) -> bool {
        self.monitor.lock().is_some()
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
    /// [`SpeculativeMonitorGuard`] type, which might change in future implementations. Instead, the return
    /// value is publicly exposed as a [`Deref`] trait.
    #[inline]
    fn __try_get(&self, duration: Duration) -> SpeculativeMonitorGuard<Option<T>> {
        let mut deadline = Deadline::lazy_after(duration);
        self.monitor.enter(|state| {
            if state.is_none() {
                Directive::Wait(deadline.remaining())
            } else {
                Directive::Return
            }
        });
        self.monitor.lock()
    }

    pub fn into_inner(self) -> Option<T> {
        self.monitor.into_inner()
    }
}

#[cfg(test)]
mod tests;