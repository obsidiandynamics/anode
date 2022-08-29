use std::time::Duration;
use crate::deadline::Deadline;
use crate::monitor::{Directive, Monitor, SpeculativeMonitor};
use crate::xlock::Moderator;

#[derive(Debug)]
pub struct ReadBiased;

pub struct ReadBiasedSync {
    monitor: SpeculativeMonitor<ReadBiasedState>,
}

#[derive(Debug)]
struct ReadBiasedState {
    readers: u32,
    writer: bool,
}

impl Moderator for ReadBiased {
    type Sync = ReadBiasedSync;

    #[inline]
    fn new() -> Self::Sync {
        Self::Sync {
            monitor: SpeculativeMonitor::new(ReadBiasedState { readers: 0, writer: false }),
        }
    }

    #[inline]
    fn try_read(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut acquired = false;
        sync.monitor.enter(|state| {
            if !acquired && !state.writer {
                acquired = true;
                state.readers += 1;
            }

            if acquired {
                Directive::Return
            } else {
                Directive::Wait(deadline.remaining())
            }
        });
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
                1 => Directive::NotifyAll,
                0 => Directive::NotifyOne,
                _ => Directive::Return
            }
        });
    }

    #[inline]
    fn try_write(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut acquired = false;
        sync.monitor.enter(|state| {
            if !acquired && state.readers == 0 && !state.writer {
                acquired = true;
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

            Directive::NotifyOne
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
                debug_assert!(state.readers > 0, "readers: {}", state.readers);
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