use std::fmt;
use std::ops::{Deref, DerefMut};
use crate::spin_mutex::{SpinGuard, SpinMutex};
use crate::remedy;
use crate::remedy::Remedy;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

pub trait Monitor<S: ?Sized> {
    fn enter<F: FnMut(&mut S) -> Directive>(&self, f: F);

    /// Invokes the given closure exactly once, supplying the encapsulated state for alteration
    /// or observation.
    ///
    /// When there is no need to wait for or notify other threads, this method is preferred
    /// over [`enter`](Self::enter), as it takes a stronger form of closure that is evaluated once.
    ///
    /// # Examples
    /// ```
    /// use anode::monitor::{Monitor, SpeculativeMonitor};
    /// struct State {
    ///     foo: u64
    /// }
    /// let monitor = SpeculativeMonitor::new(State { foo: 42 });
    /// let mut foo = None;
    /// monitor.alter(|state| {
    ///     foo = Some(state.foo);
    ///     state.foo *= 1;
    /// });
    /// assert_eq!(Some(42), foo);
    /// ```
    fn alter<F: FnOnce(&mut S)>(&self, f: F);

    /// Performs some computation over the encapsulated state. It may be as simple as
    /// extracting a value.
    ///
    /// # Examples
    /// ```
    /// use anode::monitor::{Monitor, SpeculativeMonitor};
    /// struct State {
    ///     foo: u64,
    ///     bar: u64,
    /// }
    /// let monitor = SpeculativeMonitor::new(State { foo: 42, bar: 24 });
    /// let foo = monitor.compute(|state| state.foo + state.bar);
    /// assert_eq!(66, foo);
    /// ```
    fn compute<T, F: FnOnce(&S) -> T>(&self, f: F) -> T {
        let mut val = None;
        self.alter(|state| {
            val = Some(f(state));
        });
        val.unwrap() // guaranteed to be initialised
    }
}

pub enum Directive {
    Return,
    Wait(Duration),
    NotifyOne,
    NotifyAll
}

struct Tracker<S: ?Sized> {
    waiting: u32,
    data: S,
}

pub struct SpeculativeMonitor<S: ?Sized> {
    mutex: Mutex<()>,
    cond: Condvar,
    tracker: SpinMutex<Tracker<S>>,
}

impl<S: Default> Default for SpeculativeMonitor<S> {
    fn default() -> Self {
        Self::new(S::default())
    }
}

impl<S> SpeculativeMonitor<S> {
    #[inline(always)]
    pub fn new(s: S) -> Self {
        Self {
            tracker: SpinMutex::new(Tracker {
                data: s,
                waiting: 0,
            }),
            mutex: Mutex::new(()),
            cond: Default::default(),
        }
    }

    pub fn into_inner(self) -> S {
        self.tracker.into_inner().data
    }
}

impl<S: ?Sized> SpeculativeMonitor<S> {
    pub fn num_waiting(&self) -> u32 {
        self.tracker.lock().waiting
    }

    pub fn lock(&self) -> SpeculativeMonitorGuard<S> {
        SpeculativeMonitorGuard {
            inner: self.tracker.lock()
        }
    }
}

impl<S: ?Sized> Monitor<S> for SpeculativeMonitor<S> {
    #[inline(always)]
    fn enter<F: FnMut(&mut S) -> Directive>(&self, mut f: F) {
        let mut mutex_guard = None;
        let mut woken = false;
        loop {
            let mut spin_guard = self.tracker.lock();
            if woken {
                woken = false;
                spin_guard.waiting -= 1;
            }
            let data = &mut spin_guard.data;
            let directive = f(data);
            match directive {
                Directive::Return => {
                    return;
                }
                Directive::Wait(duration) => {
                    match mutex_guard.take() {
                        None => {
                            // println!("init lock");
                            drop(spin_guard);
                            mutex_guard = Some(self.mutex.lock().remedy());
                        }
                        Some(guard) => {
                            spin_guard.waiting += 1;
                            drop(spin_guard);

                            let (guard, timed_out) =
                                remedy::cond_wait_remedy(&self.cond, guard, duration);

                            if timed_out {
                                // println!("timed out");
                                let mut spin_guard = self.tracker.lock();
                                spin_guard.waiting -= 1;
                                return;
                            } else {
                                // println!("keep going");
                                mutex_guard = Some(guard);
                                woken = true;
                            }
                        }
                    }
                }
                Directive::NotifyOne | Directive::NotifyAll => {
                    if spin_guard.waiting > 0 {
                        drop(spin_guard);
                        match mutex_guard.take() {
                            None => {
                                // println!("init lock");
                                mutex_guard = Some(self.mutex.lock().remedy());
                            }
                            Some(guard) => {
                                drop(guard);
                                match directive {
                                    Directive::NotifyOne => {
                                        self.cond.notify_one();
                                    }
                                    Directive::NotifyAll => {
                                        self.cond.notify_all();
                                    }
                                    _ => unreachable!()
                                }
                                return;
                            }
                        }
                    } else {
                        return;
                    }
                }
            }
        }
    }

    #[inline(always)]
    fn alter<F: FnOnce(&mut S)>(&self, f: F) {
        let mut spin_guard = self.tracker.lock();
        f(&mut spin_guard.data);
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for SpeculativeMonitor<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("SpeculativeMonitor");
        match self.tracker.try_lock() {
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
                d.field("data", &&(*guard).data);
            }
        }
        d.finish_non_exhaustive()
    }
}

pub struct SpeculativeMonitorGuard<'a, S: ?Sized> {
    inner: SpinGuard<'a, Tracker<S>>
}

impl<'a, S> Deref for SpeculativeMonitorGuard<'a, S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.inner.data
    }
}

impl<'a, S> DerefMut for SpeculativeMonitorGuard<'a, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner.data
    }
}

#[cfg(test)]
mod tests;
