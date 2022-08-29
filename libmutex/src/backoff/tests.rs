use std::time::Duration;
use rand::{Rng, thread_rng};
use crate::backoff::{ExpBackoff, ExpBackoffAction, NonzeroDuration};
use crate::inf_iterator::{InfIterator, IntoInfIterator};
use crate::rand::Rand64;

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

    let mut it = eb.into_inf_iter();
    assert_eq!(ExpBackoffAction::Nop, it.next());
    assert_eq!(ExpBackoffAction::Nop, it.next());
    assert_eq!(ExpBackoffAction::Yield, it.next());
    assert_eq!(ExpBackoffAction::Yield, it.next());
    assert_eq!(ExpBackoffAction::Yield, it.next());
    assert_eq!(ExpBackoffAction::Sleep(Duration::from_micros(1).into()), it.next());
    assert_eq!(ExpBackoffAction::Sleep(Duration::from_micros(2).into()), it.next());
    assert_eq!(ExpBackoffAction::Sleep(Duration::from_micros(4).into()), it.next());
    assert_eq!(ExpBackoffAction::Sleep(Duration::from_micros(8).into()), it.next());
    assert_eq!(ExpBackoffAction::Sleep(Duration::from_micros(16).into()), it.next());
    assert_eq!(ExpBackoffAction::Sleep(Duration::from_micros(30).into()), it.next());
    assert_eq!(ExpBackoffAction::Sleep(Duration::from_micros(30).into()), it.next());

    let mut it = eb.into_inf_iter();
    assert_eq!(ExpBackoffAction::Nop, it.next());
}

impl<R: Rng> Rand64 for R {
    fn next_u64(&mut self) -> u64 {
        self.next_u64()
    }
}

#[test]
fn exp_backoff_act() {
    let mut thread_rng = thread_rng();
    ExpBackoffAction::Nop.act(|| &mut thread_rng);
    ExpBackoffAction::Yield.act(|| &mut thread_rng);
    ExpBackoffAction::Sleep(Duration::from_micros(10).into()).act(|| &mut thread_rng);
}