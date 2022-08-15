use std::io::{stdin, stdout, Write};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;
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
    let num_readers = 1_000;
    let num_writers = 100;
    let num_downgraders = 100;
    let iterations = 100;
    let debug = false;
    // let num_readers = 0;
    // let num_writers = 0;
    // let num_downgraders = 2;
    // let iterations = 1_000;
    let sleep_time = Duration::from_millis(0);

    let protected = Arc::new(UrwLock::new(0));

    let mut threads = vec![];
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

    for thread in threads {
        thread.join().unwrap();
    }

    assert_eq!((num_writers + num_downgraders) * iterations, *protected.read().unwrap());
}
