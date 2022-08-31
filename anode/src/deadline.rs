use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum Deadline {
    Point(Instant),
    Forever,
    Uninitialized(Duration),
    Elapsed,
}

impl Deadline {
    #[inline(always)]
    pub fn lazy_after(duration: Duration) -> Self {
        Self::Uninitialized(duration)
    }

    #[inline(always)]
    pub fn after(duration: Duration) -> Self {
        let mut deadline = Self::lazy_after(duration);
        deadline.ensure_initialized();
        deadline
    }

    #[inline(always)]
    fn saturating_add(instant: Instant, duration: Duration) -> Self {
        match instant.checked_add(duration) {
            None => Deadline::Forever,
            Some(instant) => Deadline::Point(instant),
        }
    }

    #[inline(always)]
    fn ensure_initialized(&mut self) {
        if let Self::Uninitialized(duration) = self {
            if duration == &Duration::MAX {
                *self = Deadline::Forever;
            } else if duration ==  &Duration::ZERO {
                *self = Deadline::Elapsed;
            } else {
                *self = Self::saturating_add(Instant::now(), *duration);
            }
        }
    }

    #[inline(always)]
    pub fn remaining(&mut self) -> Duration {
        self.ensure_initialized();

        match self {
            Deadline::Point(instant) => *instant - Instant::now(),
            Deadline::Forever => Duration::MAX,
            Deadline::Elapsed => Duration::ZERO,
            _ => unreachable!(),
        }
    }
}
