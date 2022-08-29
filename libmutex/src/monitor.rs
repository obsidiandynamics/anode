use crate::spinlock::SpinLock;
use crate::remedy;
use crate::remedy::Remedy;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

pub trait Monitor<S> {
    fn enter<F: FnMut(&mut S) -> Directive>(&self, f: F);
}

pub enum Directive {
    Return,
    Wait(Duration),
    NotifyOne,
    NotifyAll
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

    pub fn num_waiting(&self) -> u32 {
        self.tracker.lock().waiting
    }
}

impl<S> Monitor<S> for SpeculativeMonitor<S> {
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
}

#[cfg(test)]
mod tests;
