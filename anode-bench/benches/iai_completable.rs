use std::time::Duration;
use anode::completable::Completable;
use iai::{black_box, main};

fn incomplete_try_get() {
    let completable = Completable::<()>::default();
    let maybe_completed = completable.try_get(Duration::ZERO);
    debug_assert!(maybe_completed.is_none());
    black_box(maybe_completed);
}

main!(incomplete_try_get);
