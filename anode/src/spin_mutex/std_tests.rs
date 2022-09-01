//! A test suite "borrowed" the [standard library](https://github.com/rust-lang/rust/blob/master/library/std/src/sync/mutex/tests.rs).

use crate::chalice::Chalice;
use crate::spin_mutex::SpinMutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc};
use std::thread;

#[derive(Eq, PartialEq, Debug)]
struct NonCopy(i32);

#[test]
fn smoke() {
    let m = SpinMutex::new(());
    drop(m.lock());
    drop(m.lock());
}

#[test]
fn lots_and_lots() {
    const J: u32 = 1000;
    const K: u32 = 3;

    let m = Arc::new(SpinMutex::new(0));

    fn inc(m: &SpinMutex<u32>) {
        for _ in 0..J {
            *m.lock() += 1;
        }
    }

    let (tx, rx) = channel();
    for _ in 0..K {
        let tx2 = tx.clone();
        let m2 = m.clone();
        thread::spawn(move || {
            inc(&m2);
            tx2.send(()).unwrap();
        });
        let tx2 = tx.clone();
        let m2 = m.clone();
        thread::spawn(move || {
            inc(&m2);
            tx2.send(()).unwrap();
        });
    }

    drop(tx);
    for _ in 0..2 * K {
        rx.recv().unwrap();
    }
    assert_eq!(*m.lock(), J * K * 2);
}

#[test]
fn try_lock() {
    let m = SpinMutex::new(());
    *m.try_lock().unwrap() = ();
}

#[test]
fn test_into_inner() {
    let m = SpinMutex::new(NonCopy(10));
    assert_eq!(m.into_inner(), NonCopy(10));
}

#[test]
fn test_into_inner_drop() {
    struct Foo(Arc<AtomicUsize>);
    impl Drop for Foo {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let num_drops = Arc::new(AtomicUsize::new(0));
    let m = SpinMutex::new(Foo(num_drops.clone()));
    assert_eq!(num_drops.load(Ordering::SeqCst), 0);
    {
        let _inner = m.into_inner();
        assert_eq!(num_drops.load(Ordering::SeqCst), 0);
    }
    assert_eq!(num_drops.load(Ordering::SeqCst), 1);
}

#[test]
fn test_get_mut() {
    let mut m = SpinMutex::new(NonCopy(10));
    *m.get_mut() = NonCopy(20);
    assert_eq!(m.into_inner(), NonCopy(20));
}

#[test]
fn test_mutex_arc_poison() {
    let arc = Arc::new(SpinMutex::new(Chalice::new(1)));
    let arc2 = arc.clone();
    let _ = thread::spawn(move || {
        let mut lock_guard = arc2.lock();
        let _chalice_guard = lock_guard.borrow_mut();
        panic!();
    })
    .join();
    assert!(arc.lock().is_poisoned());
}

#[test]
fn test_mutex_arc_nested() {
    // Tests nested mutexes and access
    // to underlying data.
    let arc = Arc::new(SpinMutex::new(1));
    let arc2 = Arc::new(SpinMutex::new(arc));
    let (tx, rx) = channel();
    let _t = thread::spawn(move || {
        let lock = arc2.lock();
        let lock2 = lock.lock();
        assert_eq!(*lock2, 1);
        tx.send(()).unwrap();
    });
    rx.recv().unwrap();
}

#[test]
fn test_mutex_arc_access_in_unwind() {
    let arc = Arc::new(SpinMutex::new(1));
    let arc2 = arc.clone();
    let _ = thread::spawn(move || -> () {
        struct Unwinder {
            i: Arc<SpinMutex<i32>>,
        }
        impl Drop for Unwinder {
            fn drop(&mut self) {
                *self.i.lock() += 1;
            }
        }
        let _u = Unwinder { i: arc2 };
        panic!();
    })
    .join();
    let lock = arc.lock();
    assert_eq!(*lock, 2);
}

#[test]
fn test_mutex_unsized() {
    let mutex: &SpinMutex<[i32]> = &SpinMutex::new([1, 2, 3]);
    {
        let b = &mut *mutex.lock();
        b[0] = 4;
        b[2] = 5;
    }
    let comp: &[i32] = &[4, 2, 5];
    assert_eq!(&*mutex.lock(), comp);
}
