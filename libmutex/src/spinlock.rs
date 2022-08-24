use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for SpinLock<T> {}
unsafe impl<T: ?Sized + Sync> Sync for SpinGuard<'_, T> {}

#[derive(Debug)]
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
        let mut guard = None;
        while guard.is_none() {
            guard = self.try_lock();
        }
        guard.unwrap()
    }

    #[inline]
    pub fn try_lock(&self) -> Option<SpinGuard<T>> {
        if self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok() {
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

#[cfg(test)]
mod tests;