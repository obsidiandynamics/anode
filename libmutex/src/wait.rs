use crate::deadline::Deadline;
use std::cmp::Ordering;
use std::time::Duration;
use std::{hint, thread};

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

const MAX_WAITS_BEFORE_YIELDING: u16 = 10;

impl Wait for Spin {
    #[inline(always)]
    fn wait_until<C>(mut condition: C, mut deadline: Deadline) -> WaitResult
    where
        C: FnMut() -> bool,
    {
        let mut waits = 0;
        while !condition() {
            if deadline.remaining().is_zero() {
                return Err(());
            }
            hint::spin_loop();
            if waits >= MAX_WAITS_BEFORE_YIELDING {
                thread::yield_now();
            } else {
                waits += 1;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
