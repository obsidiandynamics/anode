use std::sync::{Condvar, LockResult, MutexGuard, PoisonError};
use std::time::{Duration};

pub fn unpack<T>(result: LockResult<T>) -> (T, bool) {
    match result {
        Ok(inner) => (inner, false),
        Err(error) => (error.into_inner(), true),
    }
}

pub fn pack<T>(data: T, poisoned: bool) -> LockResult<T> {
    if poisoned {
        Err(PoisonError::new(data))
    } else {
        Ok(data)
    }
}

pub fn cond_wait<'a, T>(
    cond: &Condvar,
    guard: MutexGuard<'a, T>,
    duration: Duration,
) -> LockResult<(MutexGuard<'a, T>, bool)> {
    if duration.is_zero() {
        pack((guard, true), false)
    } else if duration == Duration::MAX {
        let (guard, poisoned) = unpack(cond.wait(guard));
        pack((guard, false), poisoned)
    } else {
        let ((guard, maybe_timed_out), poisoned) = unpack(cond.wait_timeout(guard, duration));
        pack((guard, maybe_timed_out.timed_out()), poisoned)
    }
}