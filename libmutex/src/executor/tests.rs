use std::ops::RangeInclusive;
use std::sync::{Arc, Barrier};
use crate::completable::Outcome;
use crate::executor::{Executor, ThreadPool};

#[test]
fn execute_tasks() {
    const THREADS: RangeInclusive<u16> = 1..=10;
    const TASKS: u16 = 100;

    for threads in THREADS {
        let pool = ThreadPool::new(threads, 10);
        let tasks = (0..TASKS)
            .map(|_| pool.submit(|| {}))
            .collect::<Vec<_>>();

        for task in tasks {
            assert_eq!(Outcome::Success(()), *task.get());
        }
    }
}

#[test]
fn abort_from_submitter() {
    let pool = ThreadPool::new(1, 10);
    let process_task_1 = Arc::new(Barrier::new(2));

    // submit 3 tasks: the 1st will be allowed to execute; the other two will be aborted
    let task_1 = {
        let process_task_1 = process_task_1.clone();
        pool.submit(move || {
            process_task_1.wait();
        })
    };

    let task_2 = pool.submit(|| {});
    let task_3 = pool.submit(|| {});
    task_2.complete(Outcome::Abort);
    task_3.complete(Outcome::Abort);

    // the executor is still waiting for the barrier
    assert!(!task_1.is_complete());

    // trip the barrier, resuming the executor
    process_task_1.wait();
    assert_eq!(Outcome::Abort, *task_2.get());
    assert_eq!(Outcome::Abort, *task_3.get());
}