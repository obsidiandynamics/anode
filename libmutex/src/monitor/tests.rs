use crate::monitor::{Directive, Monitor, SpeculativeMonitor};

#[test]
fn return_immediately() {
    let monitor = SpeculativeMonitor::new(0);
    let mut invocations = 0;
    monitor.enter(|val| {
        assert_eq!(0, *val);
        *val = 42;
        invocations += 1;
        Directive::Return
    });
    assert_eq!(1, invocations);

    monitor.enter(|val| {
        assert_eq!(42, *val);
        invocations += 1;
        Directive::Return
    });
    assert_eq!(2, invocations);
}