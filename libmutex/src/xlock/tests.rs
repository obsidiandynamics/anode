use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use crate::xlock::locklike::{LockBox, LockBoxSized};
use crate::xlock::{LockReadGuard, LockUpgradeOutcome, LockWriteGuard, ReadBiased, Spec, XLock};

#[test]
fn box_cycle() {
    let lock = XLock::<_, ReadBiased>::new(42);
    let boxed: LockBox<_> = Box::new(lock);

    // read -> release
    {
        let guard = boxed.read();
        assert_eq!(42, *guard);
    }

    // read -> upgrade -> downgrade -> release
    {
        let mut guard = boxed.read().upgrade();
        assert_eq!(42, *guard);
        *guard = 1911;
        assert_eq!(1911, *guard);

        let guard = guard.downgrade();
        assert_eq!(1911, *guard);
    }

    // write -> release
    {
        let mut guard = boxed.write();
        assert_eq!(1911, *guard);
        *guard = 1801;
        assert_eq!(1801, *guard);
    }

    // write -> downgrade -> release
    {
        let mut guard = boxed.write();
        assert_eq!(1801, *guard);
        *guard = 69;
        assert_eq!(69, *guard);

        let guard = guard.downgrade(); // drops old guard
        assert_eq!(69, *guard);
    }

    // try-read -> upgrade -> downgrade -> try_upgrade -> downgrade -> release
    {
        let guard = boxed.try_read(Duration::ZERO).unwrap();
        assert_eq!(69, *guard);

        let mut guard = guard.upgrade(); // drops old guard
        *guard = 1945;
        assert_eq!(1945, *guard);

        let guard = guard.downgrade(); // drops old guard
        assert_eq!(1945, *guard);

        let guard = guard.try_upgrade(Duration::ZERO); // possibly drops old guard
        assert!(guard.is_upgraded()); // in this case, definitely drops
        let mut guard = guard.upgraded().unwrap();
        *guard = 1941;
        assert_eq!(1941, *guard);

        let guard = guard.downgrade(); // drops old guard
        assert_eq!(1941, *guard);
    }

    // try-write -> downgrade -> release
    {
        let mut guard = boxed.try_write(Duration::ZERO).unwrap();
        *guard = 1983;
        assert_eq!(1983, *guard);

        let guard = guard.downgrade(); // drops old guard
        assert_eq!(1983, *guard);
    }
}

#[test]
fn box_sized_into_inner() {
    let lock = XLock::<_, ReadBiased>::new(42);
    let boxed: LockBoxSized<_> = Box::new(lock);
    assert_eq!(42, boxed.into_inner());
}

#[test]
fn micro_bench() {
    fn read_eventually<T, S: Spec>(lock: &XLock<T, S>, duration: Duration) -> LockReadGuard<T, S> {
        let mut val = None;
        while val.is_none() {
            val = lock.try_read(duration);
        }
        val.unwrap()
    }

    fn write_eventually<T, S: Spec>(lock: &XLock<T, S>, duration: Duration) -> LockWriteGuard<T, S> {
        let mut val = None;
        while val.is_none() {
            val = lock.try_write(duration);
        }
        val.unwrap()
    }

    let num_readers = 4;
    let num_writers = 4;
    let num_downgraders = 4;
    let num_upgraders = 4;
    let iterations = 1000;

    let read_timeout = Duration::MAX;//Duration::from_millis(10);
    let write_timeout = Duration::MAX;//Duration::from_millis(10);
    let upgrade_timeout = Duration::ZERO;//Duration::from_millis(1);

    let debug_locks = false;
    let debug_exits = false;
    let sleep_time = Duration::from_millis(0);

    let protected = Arc::new(XLock::<_, ReadBiased>::new(0));

    let mut threads = Vec::with_capacity(num_readers + num_writers + num_downgraders);
    let upgrade_timeouts = Arc::new(AtomicU64::default());
    let start_time = Instant::now();
    for i in 0..num_readers {
        let protected = protected.clone();
        threads.push(thread::spawn(move || {
            let mut last_val = 0;
            for _ in 0..iterations {
                {
                    let val = read_eventually(&protected, read_timeout);
                    if debug_locks { println!("reader {i} read-locked"); }
                    if *val < last_val {
                        panic!("Error in reader: value went from {last_val} to {val}", val = *val);
                    }
                    last_val = *val;
                    if debug_locks { println!("reader {i} read-unlocked"); }
                }
                thread::sleep(sleep_time);
            }
            if debug_exits { println!("reader {i} exited"); }
        }))
    }

    for i in 0..num_writers {
        let protected = protected.clone();
        threads.push(thread::spawn(move || {
            for _ in 0..iterations {
                {
                    let mut val = write_eventually(&protected, write_timeout);
                    if debug_locks { println!("writer {i} write-locked"); }
                    *val += 1;
                    if debug_locks { println!("writer {i} write-unlocked"); }
                }
                thread::sleep(sleep_time);
            }
            if debug_exits { println!("writer {i} exited"); }
        }))
    }

    for i in 0..num_downgraders {
        let protected = protected.clone();
        let i = i;
        threads.push(thread::spawn(move || {
            let mut last_val = 0;
            for _ in 0..iterations {
                {
                    let mut val = protected.write();
                    if debug_locks { println!("downgrader {i} write-locked"); }
                    *val += 1;

                    let val = val.downgrade();
                    if debug_locks { println!("downgrader {i} downgraded"); }
                    if *val < last_val {
                        panic!("Error in downgrader: value went from {last_val} to {val}", val = *val);
                    }
                    last_val = *val;
                    if debug_locks { println!("downgrader {i} read-unlocked"); }
                }
                thread::sleep(sleep_time);
            }
            if debug_exits { println!("downgrader {i} exited"); }
        }))
    }

    for i in 0..num_upgraders {
        let protected = protected.clone();
        let i = i;
        let upgrade_timeouts = upgrade_timeouts.clone();
        threads.push(thread::spawn(move || {
            let mut last_val = 0;
            for _ in 0..iterations {
                {
                    let val = read_eventually(&protected, read_timeout);
                    if debug_locks { println!("upgrader {i} read-locked"); }
                    if *val < last_val {
                        panic!("Error in reader: value went from {last_val} to {val}", val = *val);
                    }
                    last_val = *val;

                    let val = val.try_upgrade(upgrade_timeout);
                    match val {
                        LockUpgradeOutcome::Upgraded(mut val) => {
                            if debug_locks { println!("upgrader {i} upgraded"); }
                            *val += 1;
                            if debug_locks { println!("upgrader {i} write-unlocked"); }
                        },
                        LockUpgradeOutcome::Unchanged(_) => {
                            upgrade_timeouts.fetch_add(1, Ordering::Relaxed);
                            if debug_locks { println!("upgrader {i} upgrade timed out"); }
                        }
                    }
                }
                thread::sleep(sleep_time);
            }
            if debug_exits { println!("upgrader {i} exited"); }
        }))
    }

    for thread in threads {
        thread.join().unwrap();
    }
    let upgrade_timeouts = upgrade_timeouts.load(Ordering::Relaxed);
    let expected_writes = ((num_writers + num_downgraders + num_upgraders) * iterations) as u64 - upgrade_timeouts;
    assert_eq!(expected_writes, *protected.read());

    let time_taken = (Instant::now() - start_time).as_secs_f64();
    let ops = (num_readers + num_writers + 2 * num_downgraders + 2 * num_upgraders) * iterations;
    let rate = (ops as f64) / time_taken;
    println!("{ops} ops took {time_taken:.3} seconds; {rate:.3} ops/s");
    println!("upgrade timeouts: {upgrade_timeouts:?}");
}