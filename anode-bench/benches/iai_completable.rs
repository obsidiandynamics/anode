use std::time::Duration;
use anode::completable::{Completable, Completed};
use iai::{black_box, main};

fn incomplete_complete() -> Option<()> {
    let completable = Completable::default();
    let completed = completable.complete(());
    debug_assert!(completed.is_none());
    completed
}

fn incomplete_is_complete() -> bool {
    let completable = Completable::<()>::default();
    let completed = completable.is_complete();
    debug_assert!(!completed);
    completed
}

fn completed_complete() -> Option<()> {
    let completable = Completable::new(());
    let completed = completable.complete(());
    debug_assert!(completed.is_some());
    completed
}

fn completed_get() {
    let completable = Completable::new(());
    let completed = completable.get();
    black_box(completed);
}

fn incomplete_try_get() {
    let completable = Completable::<()>::default();
    let maybe_completed = completable.try_get(Duration::ZERO);
    debug_assert!(maybe_completed.is_none());
    black_box(maybe_completed);
}

main!(incomplete_complete, incomplete_is_complete, completed_complete, completed_get, incomplete_try_get);
