use std::time::Duration;
use anode::completable::{Completable};
use iai::{black_box, main};

fn incomplete_complete() -> Option<()> {
    let completable = Completable::default();
    let completed = completable.complete(());
    debug_assert!(completed.is_none());
    completed
}

fn incomplete_try_get() {
    let completable = Completable::<()>::default();
    let maybe_completed = completable.try_get(Duration::ZERO);
    debug_assert!(maybe_completed.is_none());
    black_box(maybe_completed);
}

fn incomplete_is_complete() -> bool {
    let completable = Completable::<()>::default();
    let completed = completable.is_complete();
    debug_assert!(!completed);
    completed
}

fn complete_complete() -> Option<()> {
    let completable = Completable::new(());
    let completed = completable.complete(());
    debug_assert!(completed.is_some());
    completed
}

fn complete_get() {
    let completable = Completable::new(());
    let completed = completable.get();
    black_box(completed);
}

main!(incomplete_complete, incomplete_is_complete, incomplete_try_get, complete_complete, complete_get);
