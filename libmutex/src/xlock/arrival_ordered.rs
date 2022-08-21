use std::sync::{Condvar, Mutex};
use std::time::Duration;
use crate::deadline::Deadline;
use crate::utils;
use crate::xlock::Spec;

#[derive(Debug)]
pub struct ArrivalOrdered;

#[derive(Debug)]
pub struct ArrivalOrderedSync {
    state: Mutex<ArrivalOrderedState>,
    cond: Condvar
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

impl Spec for ArrivalOrdered {
    type Sync = ArrivalOrderedSync;

    #[inline]
    fn new() -> Self::Sync {
        Self::Sync {
            state: Mutex::new(ArrivalOrderedState { readers: 0, writer: false, next_ticket: 1, serviced_tickets: 0 }),
            cond: Condvar::new()
        }
    }

    #[inline]
    fn try_read(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = utils::remedy(sync.state.lock());
        let ticket = state.take_ticket();
        while state.writer || state.serviced_tickets < ticket - 1 {
            let (mut guard, timed_out) =
                utils::cond_wait_remedy(&sync.cond, state, deadline.remaining());

            if timed_out {
                guard.serviced_tickets += 1;
                drop(guard);
                sync.cond.notify_all();
                return false
            }
            state = guard;
        }
        state.serviced_tickets += 1;
        state.readers += 1;
        drop(state);
        sync.cond.notify_all();
        true
    }

    #[inline]
    fn read_unlock(sync: &Self::Sync) {
        let mut state = utils::remedy(sync.state.lock());
        debug_assert!(state.readers > 0, "readers: {}", state.readers);
        debug_assert!(!state.writer);
        state.readers -= 1;
        let readers = state.readers;
        drop(state);
        if readers <= 1 {
            sync.cond.notify_all();
        }
    }

    #[inline]
    fn try_write(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = utils::remedy(sync.state.lock());
        let ticket = state.take_ticket();
        while state.readers != 0 || state.writer || state.serviced_tickets < ticket - 1 {
            let (mut guard, timed_out) =
                utils::cond_wait_remedy(&sync.cond, state, deadline.remaining());

            if timed_out {
                guard.serviced_tickets += 1;
                drop(guard);
                sync.cond.notify_all();
                return false;
            }
            state = guard;
        }
        state.serviced_tickets += 1;
        state.writer = true;
        drop(state);
        sync.cond.notify_all();
        true
    }

    #[inline]
    fn write_unlock(sync: &Self::Sync) {
        let mut state = utils::remedy(sync.state.lock());
        debug_assert!(state.readers == 0, "readers: {}", state.readers);
        debug_assert!(state.writer);
        state.writer = false;
        drop(state);
        sync.cond.notify_all();
    }

    fn downgrade(sync: &Self::Sync) {
        let mut state = utils::remedy(sync.state.lock());
        debug_assert!(state.readers == 0, "readers: {}", state.readers);
        debug_assert!(state.writer);
        state.readers = 1;
        state.writer = false;
        drop(state);
        sync.cond.notify_all();
    }

    fn try_upgrade(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = utils::remedy(sync.state.lock());
        debug_assert!(state.readers > 0, "readers: {}", state.readers);
        debug_assert!(!state.writer);
        while state.readers != 1 {
            let (guard, timed_out) =
                utils::cond_wait_remedy(&sync.cond, state, deadline.remaining());

            if timed_out {
                return false
            }
            state = guard;
            debug_assert!(state.readers > 0, "readers: {}", state.readers);
            debug_assert!(!state.writer);
        }
        state.readers = 0;
        state.writer = true;
        true
    }
}