use crate::deadline::Deadline;
use std::cmp::{Ordering};
use std::time::Duration;
use std::{hint, thread};
use std::ops::Range;
use crate::inf_iterator::{InfIterator, IntoInfIterator};
use crate::rand::{FIXED_DURATION, RandDuration};

pub type WaitResult = Result<(), ()>;

pub trait Wait {
    fn wait_until<C>(condition: C, deadline: Deadline) -> WaitResult
    where
        C: FnMut() -> bool;

    #[inline(always)]
    fn wait_for<C>(condition: C, duration: Duration) -> WaitResult
    where
        C: FnMut() -> bool,
    {
        Self::wait_until(condition, Deadline::lazy_after(duration))
    }

    #[inline(always)]
    fn wait_for_inequality<T, G>(
        mut lhs_f: G,
        mut cmp: impl FnMut(Ordering) -> bool,
        rhs: &T,
        duration: Duration,
    ) -> WaitResult
    where
        T: Ord,
        G: FnMut() -> T,
    {
        Self::wait_for(
            || {
                let lhs = lhs_f();
                let ord = lhs.cmp(rhs);
                cmp(ord)
            },
            duration,
        )
    }
}

pub struct Spin {}

impl Wait for Spin {
    #[inline(always)]
    fn wait_until<C>(mut condition: C, mut deadline: Deadline) -> WaitResult
    where
        C: FnMut() -> bool,
    {
        let mut rng = FIXED_DURATION;
        let mut backoff = ExpBackoff::sleepy().into_inf_iter();
        while !condition() {
            if deadline.remaining().is_zero() {
                return Err(());
            }
            hint::spin_loop();
            backoff.next().act(|| &mut rng);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub struct NonzeroDuration(Duration);

impl NonzeroDuration {
    #[inline(always)]
    pub fn new(duration: Duration) -> Self {
        assert!(duration > Duration::ZERO);
        Self(duration)
    }
}

impl Default for NonzeroDuration {
    #[inline(always)]
    fn default() -> Self {
        Self(Duration::new(0, 1))
    }
}

impl From<Duration> for NonzeroDuration {
    #[inline(always)]
    fn from(duration: Duration) -> Self {
        Self::new(duration)
    }
}

impl From<NonzeroDuration> for Duration {
    #[inline(always)]
    fn from(duration: NonzeroDuration) -> Self {
        duration.0
    }
}

#[derive(Debug, Clone)]
pub struct ExpBackoff {
    pub spin_iters: u64,
    pub yield_iters: u64,
    pub min_sleep: NonzeroDuration,
    pub max_sleep: NonzeroDuration,
}

impl ExpBackoff {
    pub fn spinny() -> Self {
        Self {
            spin_iters: u64::MAX,
            yield_iters: 0,
            min_sleep: NonzeroDuration::default(),
            max_sleep: NonzeroDuration::default()
        }
    }

    pub fn yieldy() -> Self {
        Self {
            spin_iters: 0,
            yield_iters: u64::MAX,
            min_sleep: NonzeroDuration::default(),
            max_sleep: NonzeroDuration::default()
        }
    }

    pub fn sleepy() -> Self {
        Self {
            spin_iters: 0,
            yield_iters: 0,
            min_sleep: Duration::from_micros(100).into(),
            max_sleep: Duration::from_millis(10).into()
        }
    }
}

impl IntoInfIterator for &ExpBackoff {
    type Item = ExpBackoffAction;
    type IntoInfIter = ExpBackoffIter;

    #[inline(always)]
    fn into_inf_iter(self) -> Self::IntoInfIter {
        Self::IntoInfIter {
            spin_limit: self.spin_iters,
            yield_limit: self.spin_iters.saturating_add(self.yield_iters),
            max_sleep: self.max_sleep.into(),
            iterations: 0,
            current_sleep: self.min_sleep.into(),
        }
    }
}

pub struct ExpBackoffIter {
    spin_limit: u64,
    yield_limit: u64,
    max_sleep: Duration,
    iterations: u64,
    current_sleep: Duration,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExpBackoffAction {
    Nop,
    Yield,
    Sleep(Duration),
}

impl ExpBackoffAction {
    #[inline(always)]
    pub fn act<'a, R, D>(&self, randomness: D) where R: RandDuration + 'a, D: FnOnce() -> &'a mut R  {
        match self {
            ExpBackoffAction::Nop => (),
            ExpBackoffAction::Yield => thread::yield_now(),
            ExpBackoffAction::Sleep(duration) => {
                let range = Range {
                    start: Duration::ZERO,
                    end: *duration,
                };
                let rng = randomness();
                thread::sleep(rng.gen_range(range));
            }
        }
    }
}

impl InfIterator for ExpBackoffIter {
    type Item = ExpBackoffAction;

    #[inline(always)]
    fn next(&mut self) -> Self::Item {
        self.iterations += 1;
        if self.iterations <= self.spin_limit {
            return ExpBackoffAction::Nop;
        }

        if self.iterations <= self.yield_limit {
            return ExpBackoffAction::Yield;
        }

        let current_sleep = self.current_sleep;
        let new_sleep = self.current_sleep * 2;
        self.current_sleep = if new_sleep <= self.max_sleep { new_sleep } else { self.max_sleep };
        ExpBackoffAction::Sleep(current_sleep)
    }
}

#[cfg(test)]
mod tests;
