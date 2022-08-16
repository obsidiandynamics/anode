use std::sync::{Condvar, LockResult, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

pub enum Deadline {
    Point(Instant),
    Perpetual,
    Uninitialized(Duration),
    Elapsed,
}

impl Deadline {
    pub fn after(duration: Duration) -> Self {
        Self::Uninitialized(duration)
    }

    fn saturating_add(instant: Instant, duration: Duration) -> Self {
        match instant.checked_add(duration) {
            None => Deadline::Perpetual,
            Some(instant) => Deadline::Point(instant),
        }
    }

    fn ensure_initialized(&mut self) {
        if let Self::Uninitialized(duration) = self {
            if duration == &Duration::MAX {
                *self = Deadline::Perpetual;
            } else if duration ==  &Duration::ZERO {
                *self = Deadline::Elapsed;
            } else {
                *self = Self::saturating_add(Instant::now(), *duration);
            }
        }
    }

    pub fn remaining(&mut self) -> Duration {
        self.ensure_initialized();

        match self {
            Deadline::Point(instant) => Instant::now() - *instant,
            Deadline::Perpetual => Duration::MAX,
            Deadline::Elapsed => Duration::ZERO,
            _ => unreachable!(),
        }
    }
}

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
    if duration == Duration::MAX {
        let (guard, poisoned) = unpack(cond.wait(guard));
        pack((guard, false), poisoned)
    } else {
        let ((guard, maybe_timed_out), poisoned) = unpack(cond.wait_timeout(guard, duration));
        pack((guard, maybe_timed_out.timed_out()), poisoned)
    }
}