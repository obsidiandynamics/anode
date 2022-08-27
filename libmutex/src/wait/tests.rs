use std::cmp::Ordering;
use std::time::Duration;
use crate::deadline::Deadline;
use crate::wait::{MAX_WAITS_BEFORE_YIELDING, Spin, Wait};

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