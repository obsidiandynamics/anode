use std::fmt::{Display};
use std::panic;
use std::panic::{AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use super::*;
use crate::test_utils::UnwindableRefCell;

#[test]
fn chalice_unsized() {
    let ref_chalice: &Chalice<dyn Display> = &Chalice::new("test");
    let ref_dyn_display = ref_chalice.borrow().either();
    assert_eq!("test", ref_dyn_display.to_string());
}

#[test]
fn borrow_unpoisoned() {
    let chalice = Chalice::new(42);
    assert!(!chalice.is_poisoned());

    let borrowed = chalice.borrow().unwrap();
    assert_eq!(42, *borrowed);
    assert!(!chalice.is_poisoned());
}

#[test]
fn borrow_mut_unpoisoned() {
    let mut chalice = Chalice::new(42);
    assert!(!chalice.is_poisoned());

    let borrowed_mut = chalice.borrow_mut();
    assert!(borrowed_mut.is_ok());
    {
        let mut guard = borrowed_mut.unwrap();
        assert_eq!(42, *guard.deref());
        assert_eq!(42, *guard.deref_mut());

        *guard += 27;
        assert_eq!(69, *guard.deref());
        assert_eq!(69, *guard.deref_mut());
    }

    assert!(!chalice.is_poisoned());
}

#[test]
fn borrow_poisoned_same_thread_via_mutex() {
    let chalice_mux = Mutex::new(Chalice::new(42));
    let result = panic::catch_unwind(|| {
        let mut mux_guard = chalice_mux.lock().unwrap();
        let chalice_guard = mux_guard.borrow_mut();
        if chalice_guard.is_ok() { panic!(); }
    });
    assert!(result.is_err());
    let mux_guard = chalice_mux.lock();
    assert!(mux_guard.is_err());

    let mut chalice = mux_guard.unwrap_err().into_inner();
    assert!(chalice.is_poisoned());
    let borrowed_result = chalice.borrow();
    assert!(borrowed_result.is_err());

    let borrowed_mut_result = chalice.borrow_mut();
    assert!(borrowed_mut_result.is_err());

    {
        let mut chalice_guard = borrowed_mut_result.unwrap_err().into_inner();
        chalice_guard.clear_poison();
        assert_eq!(42, *chalice_guard.deref());
        assert_eq!(42, *chalice_guard.deref_mut());

        *chalice_guard += 27;
        assert_eq!(69, *chalice_guard.deref());
        assert_eq!(69, *chalice_guard.deref_mut());
    }

    assert_eq!(69, *chalice.borrow().unwrap());
    assert!(!chalice.is_poisoned());
}

#[test]
fn borrow_poisoned_same_thread() {
    let mut chalice = Chalice::new(42);
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        let chalice_guard = chalice.borrow_mut();
        if chalice_guard.is_ok() { panic!(); }
    }));
    assert!(result.is_err());

    assert!(chalice.is_poisoned());
    let borrowed_result = chalice.borrow();
    assert!(borrowed_result.is_err());

    {
        // here we don't care if the chalice is poisoned (it is, in this case)
        let mut chalice_guard = chalice.borrow_mut().either();
        assert_eq!(42, *chalice_guard.deref_mut());
    }

    let borrowed_mut_result = chalice.borrow_mut();
    assert!(borrowed_mut_result.is_err());

    {
        let mut chalice_guard = borrowed_mut_result.unwrap_err().into_inner();
        chalice_guard.clear_poison();
        assert_eq!(42, *chalice_guard.deref());
        assert_eq!(42, *chalice_guard.deref_mut());

        *chalice_guard += 27;
        assert_eq!(69, *chalice_guard.deref());
        assert_eq!(69, *chalice_guard.deref_mut());
    }

    assert_eq!(69, *chalice.borrow().unwrap());
    assert!(!chalice.is_poisoned());

    {
        // here we don't care if the chalice is poisoned (it isn't, in this case)
        let val = chalice.borrow().either();
        assert_eq!(69, *val);
    }
}

#[test]
fn borrow_poisoned_same_thread_via_refcell() {
    let chalice_rc = UnwindableRefCell::new(Chalice::new(42));
    let result = panic::catch_unwind(|| {
        let chalice_rc = &chalice_rc;
        let mut mux_guard = chalice_rc.borrow_mut();
        let chalice_guard = mux_guard.borrow_mut();
        if chalice_guard.is_ok() {
            panic!();
        }
    });
    assert!(result.is_err());
    assert!(chalice_rc.borrow().is_poisoned());

    let mut chalice = chalice_rc.borrow_mut();
    assert!(chalice.is_poisoned());
    let borrowed_mut_result = chalice.borrow_mut();
    assert!(borrowed_mut_result.is_err());

    {
        let mut chalice_guard = borrowed_mut_result.unwrap_err().into_inner();
        chalice_guard.clear_poison();
        assert_eq!(42, *chalice_guard.deref());
        assert_eq!(42, *chalice_guard.deref_mut());

        *chalice_guard += 27;
        assert_eq!(69, *chalice_guard.deref());
        assert_eq!(69, *chalice_guard.deref_mut());
    }

    assert_eq!(69, *chalice.borrow().unwrap());
    assert!(!chalice.is_poisoned());
}

#[test]
fn borrow_poisoned_different_thread() {
    let chalice_mux = Arc::new(Mutex::new(Chalice::new(42)));
    let chalice_mux_clone = chalice_mux.clone();
    let handle = thread::spawn(move || {
        let mut mux_guard = chalice_mux_clone.lock().unwrap();
        let chalice_guard = mux_guard.borrow_mut();
        if chalice_guard.is_ok() { panic!(); }
    });
    assert!(handle.join().is_err());

    let mux_guard = chalice_mux.lock();
    assert!(mux_guard.is_err());

    let mut chalice = mux_guard.unwrap_err().into_inner();
    assert!(chalice.is_poisoned());
    let borrowed_result = chalice.borrow();
    assert!(borrowed_result.is_err());

    let borrowed_mut_result = chalice.borrow_mut();
    assert!(borrowed_mut_result.is_err());

    {
        let mut chalice_guard = borrowed_mut_result.unwrap_err().into_inner();
        chalice_guard.clear_poison();
        assert_eq!(42, *chalice_guard.deref());
        assert_eq!(42, *chalice_guard.deref_mut());

        *chalice_guard += 27;
        assert_eq!(69, *chalice_guard.deref());
        assert_eq!(69, *chalice_guard.deref_mut());
    }

    assert_eq!(69, *chalice.borrow().unwrap());
    assert!(!chalice.is_poisoned());
}