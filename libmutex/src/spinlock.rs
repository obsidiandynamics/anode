use std::cell::UnsafeCell;
use std::{fmt, hint};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::backoff::ExpBackoff;
use crate::inf_iterator::{InfIterator, IntoInfIterator};
use crate::rand::{*, LazyRand64, Xorshift};

unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}
unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}
unsafe impl<T: ?Sized + Sync> Sync for SpinGuard<'_, T> {}

pub struct SpinLock<T: ?Sized> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct SpinGuard<'a, T: ?Sized> {
    lock: &'a SpinLock<T>,
    /// Emulates !Send for the struct. (Until issue 68318 -- negative trait bounds -- is resolved.)
    __no_send: PhantomData<*const ()>,
}

impl<T> SpinLock<T> {
    #[inline]
    pub fn new(t: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(t),
        }
    }

    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<'a, T: ?Sized> Drop for SpinGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

impl<'a, T: ?Sized> Deref for SpinGuard<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T: ?Sized> DerefMut for SpinGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: ?Sized> SpinLock<T> {
    #[inline]
    pub fn lock(&self) -> SpinGuard<T> {
        // a [TTAS](https://en.wikipedia.org/wiki/Test_and_test-and-set) implementation that does not result in
        // continuous cache line invalidation
        loop {
            match self.try_lock() {
                None => {
                    let mut rng = LazyRand64::<Xorshift, _>::lazy(clock_seed);
                    // let mut rng = FIXED_DURATION;
                    let mut backoff = ExpBackoff::sleepy().into_inf_iter();
                    while self.locked.load(Ordering::Relaxed) {
                        hint::spin_loop();
                        backoff.next().act(|| &mut rng)
                    }
                }
                Some(guard) => return guard,
            }
        }
    }

    #[inline]
    pub fn try_lock(&self) -> Option<SpinGuard<T>> {
        if self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Acquire).is_ok() {
            Some(SpinGuard {
                lock: self,
                __no_send: PhantomData::default()
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the [`SpinLock`] mutably, no actual locking needs to
    /// take place---the mutable borrow statically guarantees no locks exist.
    #[inline]
    pub fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for SpinLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("SpinLock");
        match self.try_lock() {
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
mod tests;

#[cfg(test)]
mod std_tests;