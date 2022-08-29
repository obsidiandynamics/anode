use std::sync::{Condvar, Mutex};
use std::time::Duration;
use crate::deadline::Deadline;
use crate::remedy;
use crate::remedy::Remedy;
use crate::xlock::Moderator;

#[derive(Debug)]
pub struct WriteBiased;

#[derive(Debug)]
pub struct WriteBiasedSync {
    state: Mutex<WriteBiasedState>,
    cond: Condvar
}

#[derive(Debug)]
struct WriteBiasedState {
    readers: u32,
    writer: bool,
    writer_pending: bool,
}

impl Moderator for WriteBiased {
    type Sync = WriteBiasedSync;

    #[inline]
    fn new() -> Self::Sync {
        Self::Sync {
            state: Mutex::new(WriteBiasedState { readers: 0, writer: false, writer_pending: false }),
            cond: Condvar::new()
        }
    }

    #[inline]
    fn try_read(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut state = sync.state.lock().remedy();
        let was_writer_pending = state.writer_pending;
        while state.writer  || (was_writer_pending && state.writer_pending) {
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
        if readers <= 1 {
            sync.cond.notify_all();
        }
    }

    #[inline]
    fn try_write(sync: &Self::Sync, duration: Duration) -> bool {
        let mut deadline = Deadline::lazy_after(duration);
        let mut self_writer_pending = false;
        let mut state = sync.state.lock().remedy();
        while state.readers != 0 || state.writer {
            if !state.writer_pending {
                self_writer_pending = true;
                state.writer_pending = true;
            }

            let (mut guard, timed_out) =
                remedy::cond_wait_remedy(&sync.cond, state, deadline.remaining());

            if timed_out {
                if self_writer_pending {
                    guard.writer_pending = false;
                    drop(guard);
                    sync.cond.notify_all();
                }
                return false;
            }
            state = guard;
        }
        if self_writer_pending {
            debug_assert!(state.writer_pending);
            state.writer_pending = false;
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
        sync.cond.notify_all();
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
        let mut self_writer_pending = false;
        let mut state = sync.state.lock().remedy();
        debug_assert!(state.readers > 0, "readers: {}", state.readers);
        debug_assert!(!state.writer);
        while state.readers != 1 {
            let (mut guard, timed_out) =
                remedy::cond_wait_remedy(&sync.cond, state, deadline.remaining());

            if timed_out {
                if self_writer_pending {
                    guard.writer_pending = false;
                    drop(guard);
                    sync.cond.notify_all();
                }
                return false
            }
            if !guard.writer_pending {
                self_writer_pending = true;
                guard.writer_pending = true;
            }
            state = guard;
            debug_assert!(state.readers > 0, "readers: {}", state.readers);
            debug_assert!(!state.writer);
        }
        if self_writer_pending {
            debug_assert!(state.writer_pending);
            state.writer_pending = false;
        }
        state.readers = 0;
        state.writer = true;
        true
    }
}

#[cfg(test)]
mod tests;
