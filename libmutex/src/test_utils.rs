use crate::multilock::Fairness;
use std::cell::{Ref, RefCell, RefMut};
use std::fmt::{Debug, Formatter};
use std::panic::RefUnwindSafe;
use std::sync::{Arc};
use std::thread::JoinHandle;
use std::time::Duration;
use std::{fmt, thread};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::deadline::Deadline;
use crate::wait;
use crate::wait::Wait;

// Constants used for waiting in tests.rs.
pub const SHORT_WAIT: Duration = Duration::from_micros(1);
pub const LONG_WAIT: Duration = Duration::from_secs(10);
pub const CHECK_WAIT: Duration = Duration::from_millis(5);

pub const FAIRNESS_VARIANTS: [FairnessVariant; 3] = [
    FairnessVariant(Fairness::ReadBiased),
    FairnessVariant(Fairness::WriteBiased),
    FairnessVariant(Fairness::ArrivalOrdered),
];

pub struct FairnessVariant(Fairness);

impl From<FairnessVariant> for Fairness {
    fn from(fv: FairnessVariant) -> Self {
        println!("test running with fairness {:?}", fv.0);
        fv.0
    }
}

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
/// that the thread has blocked. Nonetheless, by the time [`spawn_blocked`] returns,
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
    let started = Arc::new(AtomicBool::new(false));
    let thread = {
        let started = started.clone();
        thread::spawn(move || {
            started.store(true, Ordering::Relaxed);
            f()
        })
    };
    wait::Spin::wait_until(|| started.load(Ordering::Relaxed), Deadline::Forever).unwrap();
    thread
}

pub struct NoPrettyPrint<T: Debug>(pub T);

impl<T: Debug> Debug for NoPrettyPrint<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // {:#?} never used here even if dbg!() specifies it
        write!(f, "{:?}", self.0)
    }
}

pub trait Addable: Send + Sync {
    fn get(&self) -> i64;

    fn add(&self, amount: i64) -> Self;
}

#[derive(Debug)]
pub struct BoxedInt(Box<i64>);

impl BoxedInt {
    pub fn new(v: i64) -> Self {
        Self(Box::new(v))
    }
}

impl Addable for BoxedInt {
    fn get(&self) -> i64 {
        *self.0
    }

    fn add(&self, amount: i64) -> Self {
        let current = self.get();
        Self::new(current + amount)
    }
}

impl Addable for i64 {
    fn get(&self) -> i64 {
        *self
    }

    fn add(&self, amount: i64) -> Self {
        self + amount
    }
}

impl Addable for String {
    fn get(&self) -> i64 {
        self.parse().unwrap()
    }

    fn add(&self, amount: i64) -> Self {
        let current = self.get();
        (current + amount).to_string()
    }
}
