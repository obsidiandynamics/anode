use std::ops::{Deref, DerefMut};
use std::thread;

#[derive(Debug)]
pub struct Chalice<T: ?Sized>{
    poisoned: bool,
    inner: T,
}

#[derive(Debug)]
pub struct MutGuard<'a, T: ?Sized> {
    chalice: &'a mut Chalice<T>
}

impl<T: ?Sized> Drop for MutGuard<'_, T> {
    fn drop(&mut self) {
        if thread::panicking() {
            self.chalice.poison();
        }
    }
}

impl<T> MutGuard<'_, T> {
    pub fn is_poisoned(&self) -> bool {
        self.chalice.is_poisoned()
    }

    pub fn clear_poison(&mut self) {
        self.chalice.clear_poison()
    }
}

impl<T: ?Sized> Deref for MutGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.chalice.inner
    }
}

impl<T: ?Sized> DerefMut for MutGuard<'_, T> {
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

pub type ChaliceResult<T> = Result<T, Poisoned<T>>;

pub trait ChaliceResultExt<T> {
    fn either(self) -> T;
}

impl<T> ChaliceResultExt<T> for ChaliceResult<T> {
    fn either(self) -> T {
        match self {
            Ok(t) => t,
            Err(Poisoned(t)) => t
        }
    }
}

impl<T: ?Sized> Chalice<T> {
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    pub fn clear_poison(&mut self) {
        self.poisoned = false
    }

    pub fn borrow(&self) -> ChaliceResult<&T> {
        if self.poisoned {
            Err(Poisoned(&self.inner))
        } else {
            Ok(&self.inner)
        }
    }

    pub fn borrow_mut(&mut self) -> ChaliceResult<MutGuard<'_, T>> {
        let poisoned = self.poisoned;
        let guard = MutGuard { chalice: self };
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