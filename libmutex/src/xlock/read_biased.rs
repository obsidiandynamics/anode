use std::sync::{Condvar, Mutex};
use std::time::Duration;
use crate::deadline::Deadline;
use crate::utils;
use crate::xlock::Spec;

#[derive(Debug)]
pub struct ReadBiased;

#[derive(Debug)]
pub struct ReadBiasedSync {
    state: Mutex<ReadBiasedState>,
    cond: Condvar
}

#[derive(Debug)]
struct ReadBiasedState {
    readers: u32,
    writer: bool,
}

impl Spec for ReadBiased {
    type Sync = ReadBiasedSync;

    #[inline]
    fn new() -> Self::Sync {
        Self::Sync {
            state: Mutex::new(ReadBiasedState { readers: 0, writer: false }),
            cond: Condvar::new()
        }
    }

    #[inline]
    fn try_read(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = utils::remedy(sync.state.lock());
        while state.writer {
            let (guard, timed_out) =
                utils::cond_wait_remedy(&sync.cond, state, deadline.remaining());

            if timed_out {
                return false
            }
            state = guard;
        }
        state.readers += 1;
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
        if readers == 1 {
            sync.cond.notify_all();
        } else if readers == 0 {
            sync.cond.notify_one()
        }
    }

    #[inline]
    fn try_write(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = utils::remedy(sync.state.lock());
        while state.readers != 0 || state.writer {
            let (guard, timed_out) =
                utils::cond_wait_remedy(&sync.cond, state, deadline.remaining());

            if timed_out {
                return false;
            }
            state = guard;
        }
        state.writer = true;
        true
    }

    #[inline]
    fn write_unlock(sync: &Self::Sync) {
        let mut state = utils::remedy(sync.state.lock());
        debug_assert!(state.readers == 0, "readers: {}", state.readers);
        debug_assert!(state.writer);
        state.writer = false;
        drop(state);
        sync.cond.notify_one();
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