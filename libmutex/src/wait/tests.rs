use std::cmp::Ordering;
use std::ops::Range;
use std::time::Duration;
use rand::{Rng, thread_rng};
use crate::deadline::Deadline;
use crate::wait::{ExpBackoff, ExpBackoffAction, MAX_WAITS_BEFORE_YIELDING, NonzeroDuration, RandomDuration, Spin, Wait};

#[test]
fn spin_once_on_elapsed_deadline() {
    let mut invocations = 0;
    let result = Spin::wait_until(|| {
        invocations += 1;
        false
    }, Deadline::Elapsed);
    assert!(result.is_err());
    assert_eq!(1, invocations);
}

#[test]
fn spin_no_yield() {
    let mut invocations = 0;
    let result = Spin::wait_until(|| {
        invocations += 1;
        invocations == MAX_WAITS_BEFORE_YIELDING + 1
    }, Deadline::Forever);
    assert!(result.is_ok());
    assert_eq!(MAX_WAITS_BEFORE_YIELDING + 1, invocations);
}

#[test]
fn spin_with_yield() {
    let mut invocations = 0;
    let result = Spin::wait_until(|| {
        invocations += 1;
        invocations == MAX_WAITS_BEFORE_YIELDING + 2
    }, Deadline::Forever);
    assert!(result.is_ok());
    assert_eq!(MAX_WAITS_BEFORE_YIELDING + 2, invocations);
}

#[test]
fn wait_for_inequality() {
    let result = Spin::wait_for_inequality(|| 69, Ordering::is_eq, &42, Duration::ZERO);
    assert!(result.is_err());

    let result = Spin::wait_for_inequality(|| 69, Ordering::is_lt, &70, Duration::ZERO);
    assert!(result.is_ok());
}

#[test]
#[should_panic]
fn nonzero_duration_panics_on_zero() {
    NonzeroDuration::new(Duration::ZERO);
}

#[test]
fn nonzero_duration_from() {
    let duration = Duration::from_micros(10);
    let nz_duration: NonzeroDuration = duration.into();
    assert_eq!(duration, nz_duration.into());
}

#[test]
fn exp_backoff() {
    let eb = ExpBackoff {
        spin_iters: 2,
        yield_iters: 3,
        min_sleep: Duration::from_micros(1).into(),
        max_sleep: Duration::from_micros(30).into()
    };

    let mut it = eb.into_iter();
    assert_eq!(Some(ExpBackoffAction::Nop), it.next());
    assert_eq!(Some(ExpBackoffAction::Nop), it.next());
    assert_eq!(Some(ExpBackoffAction::Yield), it.next());
    assert_eq!(Some(ExpBackoffAction::Yield), it.next());
    assert_eq!(Some(ExpBackoffAction::Yield), it.next());
    assert_eq!(Some(ExpBackoffAction::Sleep(Duration::from_micros(1).into())), it.next());
    assert_eq!(Some(ExpBackoffAction::Sleep(Duration::from_micros(2).into())), it.next());
    assert_eq!(Some(ExpBackoffAction::Sleep(Duration::from_micros(4).into())), it.next());
    assert_eq!(Some(ExpBackoffAction::Sleep(Duration::from_micros(8).into())), it.next());
    assert_eq!(Some(ExpBackoffAction::Sleep(Duration::from_micros(16).into())), it.next());
    assert_eq!(Some(ExpBackoffAction::Sleep(Duration::from_micros(30).into())), it.next());
    assert_eq!(Some(ExpBackoffAction::Sleep(Duration::from_micros(30).into())), it.next());

    let mut it = eb.into_iter();
    assert_eq!(Some(ExpBackoffAction::Nop), it.next());
}

impl<R: Rng> RandomDuration for R {
    fn gen_range(&mut self, range: Range<Duration>) -> Duration {
        self.gen_range(range)
    }
}

#[test]
fn exp_backoff_act() {
    let randomness = || thread_rng();
    ExpBackoffAction::Nop.act(randomness);
    ExpBackoffAction::Yield.act(randomness);
    ExpBackoffAction::Sleep(Duration::from_micros(10).into()).act(randomness);
}