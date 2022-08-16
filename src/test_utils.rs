use std::cell::{Ref, RefCell, RefMut};
use std::panic::RefUnwindSafe;
use std::sync::Mutex;
use std::thread::JoinHandle;

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

// pub struct BackgroundTask<T> {
//     inner: Mutex<Option<T>>,
//     handle: Option<JoinHandle<T>>
// }