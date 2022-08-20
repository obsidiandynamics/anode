use std::ops::RangeInclusive;
use std::sync::{Arc, Barrier};
use crate::completable::Outcome;
use crate::executor::{Executor, Queue, ThreadPool};

#[test]
fn unbounded_execute_tasks_via_submit() {
    const THREADS: RangeInclusive<u16> = 1..=10;
    const TASKS: u16 = 100;

    for threads in THREADS {
        let pool = ThreadPool::new(threads, Queue::Unbounded);
        let tasks = (0..TASKS)
            .map(|_| pool.submit(|| {}))
            .collect::<Vec<_>>();

        for task in tasks {
            assert_eq!(Outcome::Success(()), *task.get());
        }
    }
}

#[test]
fn bounded_execute_tasks_via_submit() {
    const THREADS: RangeInclusive<u16> = 1..=10;
    const TASKS: u16 = 100;

    for threads in THREADS {
        let pool = ThreadPool::new(threads, Queue::Bounded(10));
        let tasks = (0..TASKS)
            .map(|_| pool.submit(|| {}))
            .collect::<Vec<_>>();

        for task in tasks {
            assert_eq!(Outcome::Success(()), *task.get());
        }
    }
}

#[test]
fn unbounded_execute_tasks_via_try_submit() {
    const THREADS: RangeInclusive<u16> = 1..=10;
    const TASKS: u16 = 100;

    for threads in THREADS {
        let pool = ThreadPool::new(threads, Queue::Unbounded);
        let tasks = (0..TASKS)
            .map(|_| {
                let submission = pool.try_submit(|| {});
                submission.unwrap()
            })
            .collect::<Vec<_>>();

        for task in tasks {
            assert_eq!(Outcome::Success(()), *task.get());
        }
    }
}

#[test]
fn bounded_execute_tasks_via_try_submit() {
    const THREADS: RangeInclusive<u16> = 1..=10;
    const TASKS: u16 = 100;

    for threads in THREADS {
        let pool = ThreadPool::new(threads, Queue::Bounded(10));
        let tasks = (0..TASKS)
            .map(|_| {
                let mut submission = None;
                while submission.is_none() {
                    submission = pool.try_submit(|| {});
                }
                submission.unwrap()
            })
            .collect::<Vec<_>>();

        for task in tasks {
            assert_eq!(Outcome::Success(()), *task.get());
        }
    }
}

#[test]
fn unbounded_abort_from_submitter() {
    let pool = ThreadPool::new(1, Queue::Unbounded);
    let process_task_1 = Arc::new(Barrier::new(2));

    // submit 3 tasks: the 1st will be allowed to execute; the other two will be aborted
    let task_1 = {
        let process_task_1 = process_task_1.clone();
        pool.submit(move || {
            println!("entered");
            process_task_1.wait();
            println!("exited");
        })
    };

    let task_2 = pool.submit(|| {});
    println!("submitted task_2");
    let task_3 = pool.submit(|| {});
    println!("submitted task_3");
    task_2.complete(Outcome::Abort);
    println!("aborted task_2");
    task_3.complete(Outcome::Abort);
    println!("aborted task_3");

    // the executor is still waiting for the barrier
    assert!(!task_1.is_complete());

    // trip the barrier, resuming the executor
    println!("main tripping process_task_1");
    process_task_1.wait();
    println!("main tripped process_task_1");
    assert_eq!(Outcome::Success(()), *task_1.get());
    assert_eq!(Outcome::Abort, *task_2.get());
    assert_eq!(Outcome::Abort, *task_3.get());
}

#[test]
fn unbounded_abort_from_executor() {
    let pool = ThreadPool::new(1, Queue::Unbounded);
    let start_task_1 = Arc::new(Barrier::new(2));
    let end_task_1 = Arc::new(Barrier::new(2));

    // submit 3 tasks: the 1st will be allowed to execute; the other two will be aborted
    let task_1 = {
        let start_task_1 = start_task_1.clone();
        let end_task_1 = end_task_1.clone();
        pool.submit(move || {
            println!("entered");
            start_task_1.wait();
            println!("started");
            end_task_1.wait();
            println!("exited");
        })
    };

    let task_2 = pool.submit(|| {});
    println!("submitted task_2");
    let task_3 = pool.submit(|| {});
    println!("submitted task_3");

    // wait until the executor begins the first task
    start_task_1.wait();

    // shut down the pool, forcing the remaining tasks to abort
    drop(pool);

    // unblock the executor
    end_task_1.wait();

    assert_eq!(Outcome::Success(()), *task_1.get());
    assert_eq!(Outcome::Abort, *task_2.get());
    assert_eq!(Outcome::Abort, *task_3.get());
}

#[test]
fn bounded_queuing() {
    let pool = ThreadPool::new(1, Queue::Bounded(1));
    let start_task_1 = Arc::new(Barrier::new(2));
    let end_task_1 = Arc::new(Barrier::new(2));

    // submit 3 tasks: the 1st will be allowed to execute; the other two will be aborted
    let task_1 = {
        let start_task_1 = start_task_1.clone();
        let end_task_1 = end_task_1.clone();
        pool.submit(move || {
            println!("entered");
            start_task_1.wait();
            println!("started");
            end_task_1.wait();
            println!("exited");
        })
    };

    // wait for the executor to task_1, ensuring that the channel now has capacity
    start_task_1.wait();

    // the executor is still waiting for the barrier
    assert!(!task_1.is_complete());

    // task_2 can be submitted within the queue's capacity
    let task_2 = pool.try_submit(|| {}).unwrap();
    println!("submitted task_2");
    assert!(!task_1.is_complete());

    // task_3 cannot be submitted at this stage
    let task_3 = pool.try_submit(|| {});
    assert!(task_3.is_none());

    // unblock the executor, letting it advance to task_2
    end_task_1.wait();

    // task_3 may now be submitted
    let task_3 = pool.submit(|| {});

    assert_eq!(Outcome::Success(()), *task_1.get());
    assert_eq!(Outcome::Success(()), *task_2.get());
    assert_eq!(Outcome::Success(()), *task_3.get());
}