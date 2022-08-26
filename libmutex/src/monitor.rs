use crate::spinlock::SpinLock;
use crate::utils;
use crate::utils::Remedy;
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Duration;

pub trait Monitor<S> {
    fn enter<F: FnMut(&mut S) -> Directive>(&self, f: F);
}

pub enum Directive {
    Return,
    Wait(Duration),
}

struct Tracker<S> {
    data: S,
    waiting: u32,
}

pub struct SpeculativeMonitor<S> {
    tracker: SpinLock<Tracker<S>>,
    mutex: Mutex<()>,
    cond: Condvar,
}

impl<S> SpeculativeMonitor<S> {
    pub fn new(s: S) -> Self {
        Self {
            tracker: SpinLock::new(Tracker {
                data: s,
                waiting: 0,
            }),
            mutex: Mutex::new(()),
            cond: Default::default(),
        }
    }
}

impl<S> Monitor<S> for SpeculativeMonitor<S> {
    fn enter<F: FnMut(&mut S) -> Directive>(&self, mut f: F) {
        let mut mutex_guard: Option<MutexGuard<()>> = None;
        loop {
            let mut spin_guard = self.tracker.lock();
            let data = &mut spin_guard.data;
            let directive = f(data);
            match directive {
                Directive::Return => {
                    return;
                }
                Directive::Wait(duration) => {
                    match mutex_guard.take() {
                        None => {
                            mutex_guard = Some(self.mutex.lock().remedy());
                        }
                        Some(guard) => {
                            spin_guard.waiting += 1;
                            drop(spin_guard);

                            let (guard, timed_out) =
                                utils::cond_wait_remedy(&self.cond, guard, duration);

                            let mut spin_guard = self.tracker.lock();
                            spin_guard.waiting -= 1;
                            drop(spin_guard);

                            if timed_out {
                                return;
                            } else {
                                mutex_guard = Some(guard);
                            }
                        }
                    }
                }
            }
        }
    }

    // fn enter<F: FnMut(&mut S) -> Directive>(&self, mut f: F) {
    //     let mut slow = false;
    //     let mut mutex_guard = None;
    //     loop {
    //         mutex_guard = if slow && mutex_guard.is_none() { Some(self.mutex.lock().remedy()) } else { None };
    //         let mut spin_guard = self.tracker.lock();
    //         let data = &mut spin_guard.data;
    //         let directive = f(data);
    //         match directive {
    //             Directive::Return => { return; }
    //             Directive::Wait(duration) => {
    //                 spin_guard.waiting += 1;
    //                 drop(spin_guard);
    //                 if slow {
    //                     let guard = mutex_guard.take().unwrap();
    //                     let (guard, timed_out) = utils::cond_wait_remedy(&self.cond, guard, duration);
    //                     let mut spin_guard = self.tracker.lock();
    //                     spin_guard.waiting -= 1;
    //                     drop(spin_guard);
    //
    //                     if timed_out {
    //                         return;
    //                     } else {
    //                         mutex_guard = Some(guard);
    //                     }
    //                 } else {
    //                     slow = true;
    //                 }
    //             }
    //         }
    //     }
    // }
}

#[cfg(test)]
mod tests;
