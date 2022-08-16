use std::cell::{Ref, RefCell, RefMut};
use std::panic::RefUnwindSafe;

pub struct UnwindableRefCell<T: ?Sized> {
    inner: RefCell<T>,
}

impl<T> UnwindableRefCell<T> {
    pub fn new(t: T) -> Self {
        Self { inner: RefCell::new(t) }
    }

    pub fn into_inner(self) -> T {
        self.inner.into_inner()
    }
}

impl<T: ?Sized> UnwindableRefCell<T> {
    pub fn borrow(&self) -> Ref<T> {
        self.inner.borrow()
    }

    pub fn borrow_mut(&self) -> RefMut<T> {
        self.inner.borrow_mut()
    }
}

impl<T> RefUnwindSafe for UnwindableRefCell<T> {}