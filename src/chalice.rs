use std::ops::{Deref, DerefMut};
use std::panic::{RefUnwindSafe, UnwindSafe};
use std::thread;

#[derive(Debug)]
pub struct Chalice<T: ?Sized>{
    poisoned: bool,
    inner: T,
}

// impl<T: ?Sized> UnwindSafe for Chalice<T> {}
// impl<T: ?Sized> RefUnwindSafe for Chalice<T> {}

#[derive(Debug)]
pub struct WriteGuard<'a, T: ?Sized> {
    chalice: &'a mut Chalice<T>
}

impl<T: ?Sized> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        if thread::panicking() {
            self.chalice.poison();
        }
    }
}

impl<T> WriteGuard<'_, T> {
    fn is_poisoned(&self) -> bool {
        self.chalice.is_poisoned()
    }

    fn clear_poison(&mut self) {
        self.chalice.clear_poison()
    }
}

impl<T: ?Sized> Deref for WriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.chalice.inner
    }
}

impl<T: ?Sized> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.chalice.inner
    }
}

#[derive(Debug)]
pub struct Poisoned<T: ?Sized>(T);

impl<T> Poisoned<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Chalice<T> {
    pub fn new(t: T) -> Self {
        Self { inner: t, poisoned: false }
    }
}

impl<T: ?Sized> Chalice<T> {
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    pub fn clear_poison(&mut self) {
        self.poisoned = false
    }

    pub fn borrow(&self) -> Result<&T, Poisoned<&T>> {
        if self.poisoned {
            Err(Poisoned(&self.inner))
        } else {
            Ok(&self.inner)
        }
    }

    pub fn borrow_mut(&mut self) -> Result<WriteGuard<'_, T>, Poisoned<WriteGuard<'_, T>>> {
        let poisoned = self.poisoned;
        let guard = WriteGuard { chalice: self };
        if poisoned {
            Err(Poisoned(guard))
        } else {
            Ok(guard)
        }
    }

    fn poison(&mut self) {
        self.poisoned = true;
    }
}

#[cfg(test)]
mod tests;