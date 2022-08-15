use std::io::{stdin, stdout, Write};
use std::ops::Sub;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use libmutex::urw_lock::UrwLock;

fn main() {
    // let pair = Arc::new((Mutex::new(false), Condvar::new()));
    // let pair2 = pair.clone();
    //
    // // Inside of our lock, spawn a new thread, and then wait for it to start.
    // thread::spawn(move || {
    //     thread::sleep(Duration::from_millis(1_000));
    //     println!("changing");
    //     let &(ref lock, ref cvar) = &*pair2;
    //     let mut started = lock.lock().unwrap();
    //     *started = true;
    //     // We notify the condvar that the value has changed.
    //     cvar.notify_one();
    // });
    //
    // // Wait for the thread to start up.
    // let &(ref lock, ref cvar) = &*pair;
    // let mut started = lock.lock().unwrap();
    // while !*started {
    //     println!("waiting");
    //     started = cvar.wait(started).unwrap();
    // }
    let num_readers = 8;
    let num_writers = 8;
    let num_downgraders = 8;
    let num_upgraders = 1;
    let iterations = 1_000;
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
                    let val = protected.read().unwrap();
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
            for _ in 0..iterations {
                {
                    let mut val = protected.write().unwrap();
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
                    let val = protected.read().unwrap();
                    if debug { println!("upgrader {i} read-locked"); }
                    if *val < last_val {
                        panic!("Error in upgrader: value went from {last_val} to {val}", val = *val);
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
