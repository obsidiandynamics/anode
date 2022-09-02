use std::time::Duration;
use anode::completable::Completable;
use iai::main;

fn incomplete_try_get() {
    let completable = Completable::<()>::default();
    let maybe_completed = completable.try_get(Duration::ZERO);
    debug_assert!(maybe_completed.is_none());
}

main!(incomplete_try_get);
