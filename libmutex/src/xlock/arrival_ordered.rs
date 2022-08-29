use std::time::Duration;
use crate::deadline::Deadline;
use crate::monitor::{Directive, Monitor, SpeculativeMonitor};
use crate::xlock::Moderator;

#[derive(Debug)]
pub struct ArrivalOrdered;

pub struct ArrivalOrderedSync {
    monitor: SpeculativeMonitor<ArrivalOrderedState>,
}

#[derive(Debug)]
struct ArrivalOrderedState {
    readers: u32,
    writer: bool,
    next_ticket: u64,
    serviced_tickets: u64
}

impl ArrivalOrderedState {
    #[inline]
    fn take_ticket(&mut self) -> u64 {
        let next = self.next_ticket;
        self.next_ticket = next + 1;
        next
    }
}

impl Moderator for ArrivalOrdered {
    type Sync = ArrivalOrderedSync;

    #[inline]
    fn new() -> Self::Sync {
        Self::Sync {
            monitor: SpeculativeMonitor::new(ArrivalOrderedState { readers: 0, writer: false, next_ticket: 1, serviced_tickets: 0 }),
        }
    }

    #[inline]
    fn try_read(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut acquired = false;
        let mut ticket = 0;
        sync.monitor.enter(|state| {
            if ticket == 0 {
                ticket = state.take_ticket();
            }
            if !acquired && !state.writer && state.serviced_tickets >= ticket - 1 {
                acquired = true;
                state.readers += 1;
                state.serviced_tickets += 1;
            }

            if acquired {
                Directive::NotifyAll
            } else {
                Directive::Wait(deadline.remaining())
            }
        });

        if !acquired {
            let mut incremented_serviced = false;
            sync.monitor.enter(|state| {
                if !incremented_serviced {
                    incremented_serviced = true;
                    state.serviced_tickets += 1;
                }
                Directive::NotifyAll
            })
        }

        acquired

        // let mut deadline = Deadline::lazy_after(duration);
        // let mut state = sync.monitor.lock().remedy();
        // let ticket = state.take_ticket();
        // while state.writer || state.serviced_tickets < ticket - 1 {
        //     let (mut guard, timed_out) =
        //         remedy::cond_wait_remedy(&sync.cond, state, deadline.remaining());
        //
        //     if timed_out {
        //         guard.serviced_tickets += 1;
        //         drop(guard);
        //         sync.cond.notify_all();
        //         return false
        //     }
        //     state = guard;
        // }
        // state.serviced_tickets += 1;
        // state.readers += 1;
        // drop(state);
        // sync.cond.notify_all();
        // true
    }

    #[inline]
    fn read_unlock(sync: &Self::Sync) {
        let mut released = false;
        sync.monitor.enter(|state| {
            if !released {
                debug_assert!(state.readers > 0, "readers: {}", state.readers);
                debug_assert!(!state.writer);

                released = true;
                state.readers -= 1;
            }

            match state.readers {
                0 | 1 => Directive::NotifyAll,
                _ => Directive::Return
            }
        });

        // let mut state = sync.monitor.lock().remedy();
        // debug_assert!(state.readers > 0, "readers: {}", state.readers);
        // debug_assert!(!state.writer);
        // state.readers -= 1;
        // let readers = state.readers;
        // drop(state);
        // if readers <= 1 {
        //     sync.cond.notify_all();
        // }
    }

    #[inline]
    fn try_write(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut acquired = false;
        let mut ticket = 0;
        sync.monitor.enter(|state| {
            if ticket == 0 {
                ticket = state.take_ticket();
            }
            if !acquired && state.readers == 0 && !state.writer && state.serviced_tickets >= ticket - 1 {
                acquired = true;
                state.writer = true;
                state.serviced_tickets += 1;
            }

            if acquired {
                Directive::NotifyAll
            } else {
                Directive::Wait(deadline.remaining())
            }
        });

        if !acquired {
            let mut incremented_serviced = false;
            sync.monitor.enter(|state| {
                if !incremented_serviced {
                    incremented_serviced = true;
                    state.serviced_tickets += 1;
                }
                Directive::NotifyAll
            })
        }

        acquired
        // let mut deadline = Deadline::lazy_after(duration);
        // let mut state = sync.monitor.lock().remedy();
        // let ticket = state.take_ticket();
        // while state.readers != 0 || state.writer || state.serviced_tickets < ticket - 1 {
        //     let (mut guard, timed_out) =
        //         remedy::cond_wait_remedy(&sync.cond, state, deadline.remaining());
        //
        //     if timed_out {
        //         guard.serviced_tickets += 1;
        //         drop(guard);
        //         sync.cond.notify_all();
        //         return false;
        //     }
        //     state = guard;
        // }
        // state.serviced_tickets += 1;
        // state.writer = true;
        // drop(state);
        // sync.cond.notify_all();
        // true
    }

    #[inline]
    fn write_unlock(sync: &Self::Sync) {
        let mut released = false;
        sync.monitor.enter(|state| {
            if !released {
                debug_assert!(state.readers == 0, "readers: {}", state.readers);
                debug_assert!(state.writer);

                released = true;
                state.writer = false;
            }

            Directive::NotifyAll
        });

        // let mut state = sync.monitor.lock().remedy();
        // debug_assert!(state.readers == 0, "readers: {}", state.readers);
        // debug_assert!(state.writer);
        // state.writer = false;
        // drop(state);
        // sync.cond.notify_all();
    }

    fn downgrade(sync: &Self::Sync) {
        let mut released = false;
        sync.monitor.enter(|state| {
            if !released {
                debug_assert!(state.readers == 0, "readers: {}", state.readers);
                debug_assert!(state.writer);

                released = true;
                state.writer = false;
                state.readers = 1;
            }

            Directive::NotifyAll
        });

        // let mut state = sync.monitor.lock().remedy();
        // debug_assert!(state.readers == 0, "readers: {}", state.readers);
        // debug_assert!(state.writer);
        // state.readers = 1;
        // state.writer = false;
        // drop(state);
        // sync.cond.notify_all();
    }

    fn try_upgrade(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut acquired = false;
        sync.monitor.enter(|state| {
            if !acquired && state.readers == 1 {
                debug_assert!(!state.writer);

                acquired = true;
                state.readers = 0;
                state.writer = true;
            }

            if acquired {
                Directive::Return
            } else {
                Directive::Wait(deadline.remaining())
            }
        });
        acquired

        // let mut deadline = Deadline::lazy_after(duration);
        // let mut state = sync.monitor.lock().remedy();
        // debug_assert!(state.readers > 0, "readers: {}", state.readers);
        // debug_assert!(!state.writer);
        // while state.readers != 1 {
        //     let (guard, timed_out) =
        //         remedy::cond_wait_remedy(&sync.cond, state, deadline.remaining());
        //
        //     if timed_out {
        //         return false
        //     }
        //     state = guard;
        //     debug_assert!(state.readers > 0, "readers: {}", state.readers);
        //     debug_assert!(!state.writer);
        // }
        // state.readers = 0;
        // state.writer = true;
        // true
    }
}

#[cfg(test)]
mod tests;