use std::sync::{Condvar, Mutex};
use std::time::Duration;
use crate::deadline::Deadline;
use crate::remedy;
use crate::remedy::Remedy;
use crate::xlock::Moderator;

#[derive(Debug)]
pub struct LegacyReadBiased;

#[derive(Debug)]
pub struct LegacyReadBiasedSync {
    state: Mutex<LegacyReadBiasedState>,
    cond: Condvar
}

#[derive(Debug)]
struct LegacyReadBiasedState {
    readers: u32,
    writer: bool,
}

impl Moderator for LegacyReadBiased {
    type Sync = LegacyReadBiasedSync;

    #[inline]
    fn new() -> Self::Sync {
        Self::Sync {
            state: Mutex::new(LegacyReadBiasedState { readers: 0, writer: false }),
            cond: Condvar::new()
        }
    }

    #[inline]
    fn try_read(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = sync.state.lock().remedy();
        while state.writer {
            let (guard, timed_out) =
                remedy::cond_wait_remedy(&sync.cond, state, deadline.remaining());

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
        let mut state = sync.state.lock().remedy();
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
        let mut state = sync.state.lock().remedy();
        while state.readers != 0 || state.writer {
            let (guard, timed_out) =
                remedy::cond_wait_remedy(&sync.cond, state, deadline.remaining());

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
        let mut state = sync.state.lock().remedy();
        debug_assert!(state.readers == 0, "readers: {}", state.readers);
        debug_assert!(state.writer);
        state.writer = false;
        drop(state);
        sync.cond.notify_one();
    }

    fn downgrade(sync: &Self::Sync) {
        let mut state = sync.state.lock().remedy();
        debug_assert!(state.readers == 0, "readers: {}", state.readers);
        debug_assert!(state.writer);
        state.readers = 1;
        state.writer = false;
        drop(state);
        sync.cond.notify_all();
    }

    fn try_upgrade(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = sync.state.lock().remedy();
        debug_assert!(state.readers > 0, "readers: {}", state.readers);
        debug_assert!(!state.writer);
        while state.readers != 1 {
            let (guard, timed_out) =
                remedy::cond_wait_remedy(&sync.cond, state, deadline.remaining());

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