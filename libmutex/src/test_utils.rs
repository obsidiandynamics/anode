use std::cell::{Ref, RefCell, RefMut};
use std::fmt::{Debug, Formatter};
use std::panic::RefUnwindSafe;
use std::sync::{Arc, Barrier};
use std::{fmt, thread};
use std::thread::JoinHandle;
use std::time::Duration;

// Constants used for waiting in tests.
pub const SHORT_WAIT: Duration = Duration::from_micros(1);
pub const LONG_WAIT: Duration = Duration::from_secs(10);
pub const CHECK_WAIT: Duration = Duration::from_millis(5);

pub struct UnwindableRefCell<T: ?Sized> {
    inner: RefCell<T>,
}

impl<T> UnwindableRefCell<T> {
    pub fn new(t: T) -> Self {
        Self {
            inner: RefCell::new(t),
        }
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

/// Spawns a new thread and waits until its closure has _started_ executing.
///
/// Useful for _probabilistically_ testing code where a thread will start off by blocking
/// on something, and we want to verify that the thread is, indeed, blocked. This function
/// only guarantees that the closure has begun executing; it doesn't guarantee
/// that the thread has blocked. Nonetheless, by the time `spawn_blocked` returns,
/// its highly likely that the thread entered the blocked state. It saves us having to
/// add a [`thread::sleep`].
///
/// # Examples (not compiled)
/// ```
/// use libmutex::test_utils::spawn_blocked;
/// let thread = spawn_blocked(|| {
///     // wait_for_something_important
/// });
/// assert!(!thread.is_finished());
/// ```
pub fn spawn_blocked<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    let barrier = Arc::new(Barrier::new(2));
    let _barrier = barrier.clone();
    let thread = thread::spawn(move || {
        _barrier.wait();
        f()
    });
    barrier.wait();
    thread
}

pub struct NoPrettyPrint<T: Debug>(pub T);

impl<T: Debug> Debug for NoPrettyPrint<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // {:#?} never used here even if dbg!() specifies it
        write!(f, "{:?}", self.0)
    }
}
