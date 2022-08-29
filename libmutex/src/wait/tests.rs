use std::cmp::Ordering;
use std::time::Duration;
use crate::deadline::Deadline;
use crate::wait::Spin;
use crate::wait::Wait;

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
fn spin_a_while() {
    const MAX_INVOCATIONS: u8 = 10;
    let mut invocations = 0;
    let result = Spin::wait_until(|| {
        invocations += 1;
        invocations == MAX_INVOCATIONS
    }, Deadline::Forever);
    assert!(result.is_ok());
    assert_eq!(MAX_INVOCATIONS, invocations);
}

#[test]
fn wait_for_inequality() {
    let result = Spin::wait_for_inequality(|| 69, Ordering::is_eq, &42, Duration::ZERO);
    assert!(result.is_err());

    let result = Spin::wait_for_inequality(|| 69, Ordering::is_lt, &70, Duration::ZERO);
    assert!(result.is_ok());
}