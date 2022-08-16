use std::ops::Deref;
use std::sync::{Arc, LockResult};
use std::thread;
use std::time::{Duration, Instant};
use libmutex::urw_lock::{UrwLock, UrwLockReadGuard, UrwLockWriteGuard};

fn main() {
    let num_readers = 8;
    let num_writers = 8;
    let num_downgraders = 8;
    let num_upgraders = 1;
    let iterations = 1_000;

    let read_timeout = Duration::MAX;//Duration::from_millis(10);
    let write_timeout = Duration::MAX;//Duration::from_millis(10);

    let debug = false;
    // let num_readers = 0;
    // let num_writers = 0;
    // let num_downgraders = 2;
    // let iterations = 1_000;
    let sleep_time = Duration::from_millis(0);

    let protected = Arc::new(UrwLock::new(0));

    let mut threads = Vec::with_capacity(num_readers + num_writers + num_downgraders);
    let start_time = Instant::now();
    for _ in 0..num_readers {
        let protected = protected.clone();
        threads.push(thread::spawn(move || {
            let mut last_val = 0;
            for _ in 0..iterations {
                {
                    let val = read_eventually(&protected, read_timeout);
                    if *val < last_val {
                        panic!("Error in reader: value went from {last_val} to {val}", val = *val);
                    }
                    last_val = *val;
                }
                thread::sleep(sleep_time);
            }
        }))
    }

    for _ in 0..num_writers {
        let protected = protected.clone();
        threads.push(thread::spawn(move || {
            for i in 0..iterations {
                {
                    let mut val = write_eventually(&protected, write_timeout);
                    if debug { println!("writer {i} write-locked"); }
                    *val += 1;
                }
                thread::sleep(sleep_time);
            }
        }))
    }

    for i in 0..num_downgraders {
        let protected = protected.clone();
        let i = i;
        threads.push(thread::spawn(move || {
            let mut last_val = 0;
            for _ in 0..iterations {
                {
                    let mut val = protected.write().unwrap();
                    if debug { println!("downgrader {i} write-locked"); }
                    *val += 1;

                    let val = val.downgrade();
                    if debug { println!("downgrader {i} downgraded"); }
                    if *val < last_val {
                        panic!("Error in downgrader: value went from {last_val} to {val}", val = *val);
                    }
                    last_val = *val;
                    if debug { println!("downgrader {i} read-unlocked"); }
                }
                thread::sleep(sleep_time);
            }
        }))
    }

    for i in 0..num_upgraders {
        let protected = protected.clone();
        let i = i;
        threads.push(thread::spawn(move || {
            let mut last_val = 0;
            for _ in 0..iterations {
                {
                    let val = read_eventually(&protected, read_timeout);
                    if debug { println!("upgrader {i} read-locked"); }
                    if *val < last_val {
                        panic!("Error in reader: value went from {last_val} to {val}", val = *val);
                    }
                    last_val = *val;

                    let mut val = val.upgrade();
                    if debug { println!("upgrader {i} upgraded"); }
                    *val += 1;
                    if debug { println!("upgrader {i} write-unlocked"); }
                }
                thread::sleep(sleep_time);
            }
        }))
    }

    for thread in threads {
        thread.join().unwrap();
    }
    let time_taken = (Instant::now() - start_time).as_secs_f64();
    assert_eq!((num_writers + num_downgraders + num_upgraders) * iterations, *protected.read().unwrap());
    let ops = (num_readers + num_writers + 2 * num_downgraders + 2 * num_upgraders) * iterations;
    let rate = (ops as f64) / time_taken;
    println!("{ops} ops took {time_taken:.3} seconds; {rate:.3} ops/s");
}

fn read_eventually<T>(lock: &UrwLock<T>, duration: Duration) -> UrwLockReadGuard<T> {
    let mut val: Option<LockResult<UrwLockReadGuard<T>>> = None;
    while val.is_none() {
        val = lock.try_read(duration);
    }
    val.unwrap().unwrap()
}

fn write_eventually<T>(lock: &UrwLock<T>, duration: Duration) -> UrwLockWriteGuard<T> {
    let mut val: Option<LockResult<UrwLockWriteGuard<T>>> = None;
    while val.is_none() {
        val = lock.try_write(duration);
    }
    val.unwrap().unwrap()
}
