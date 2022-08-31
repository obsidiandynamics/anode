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
            let mut inc_serviced = false;
            sync.monitor.enter(|state| {
                if !inc_serviced {
                    inc_serviced = true;
                    state.serviced_tickets += 1;
                }
                Directive::NotifyAll
            })
        }

        acquired
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
            let mut inc_serviced = false;
            sync.monitor.enter(|state| {
                if !inc_serviced {
                    inc_serviced = true;
                    state.serviced_tickets += 1;
                }
                Directive::NotifyAll
            })
        }

        acquired
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
    }
}

#[cfg(test)]
mod tests;