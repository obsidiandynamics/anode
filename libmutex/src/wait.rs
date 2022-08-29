use crate::deadline::Deadline;
use std::cmp::{Ordering};
use std::time::Duration;
use std::{hint};
use crate::backoff::ExpBackoff;
use crate::inf_iterator::{InfIterator, IntoInfIterator};
use crate::rand::{FIXED_DURATION};

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

#[cfg(test)]
mod tests;
