use std::ops::{Deref, DerefMut};
use std::sync::{Condvar, LockResult, Mutex, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Debug, Default)]
struct InternalState {
    readers: u32,
    writer: bool
}

pub struct UrwLock<T: ?Sized> {
    state: Mutex<InternalState>,
    cond: Condvar,
    data: RwLock<T>
}

pub struct UrwLockReadGuard<'a, T> {
    data: RwLockReadGuard<'a, T>,
    lock: &'a UrwLock<T>,
}

impl<T> Drop for UrwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.read_unlock();
    }
}

impl<T> Deref for UrwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data.deref()
    }
}

pub struct UrwLockWriteGuard<'a, T> {
    data: RwLockWriteGuard<'a, T>,
    lock: &'a UrwLock<T>,
}

impl<T> Drop for UrwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.write_unlock();
    }
}

impl<T> Deref for UrwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data.deref()
    }
}

impl<T> DerefMut for UrwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data.deref_mut()
    }
}

impl <T> UrwLock<T> {
    pub fn new(t: T) -> Self {
        Self {
            state: Mutex::new(InternalState::default()),
            cond: Condvar::new(),
            data: RwLock::new(t)
        }
    }

    pub fn read(&self) -> LockResult<UrwLockReadGuard<'_, T>> {
        let mut state = self.state.lock().unwrap();
        while state.writer {
            state = self.cond.wait(state).unwrap();
        }
        state.readers += 1;
        drop(state);

        let (data, poisoned) = unpack(self.data.read());
        let urw_guard = UrwLockReadGuard {
            data, lock: self
        };
        pack(urw_guard, poisoned)
    }

    fn read_unlock(&self) {
        let mut state = self.state.lock().unwrap();
        state.readers -= 1;
        if state.readers == 1{
            self.cond.notify_all();
        } else if state.readers == 0 {
            self.cond.notify_one();
        }
    }

    pub fn write(&self) -> LockResult<UrwLockWriteGuard<'_, T>> {
        let mut state = self.state.lock().unwrap();
        while state.readers != 0 {
            state = self.cond.wait(state).unwrap();
        }
        state.writer = true;
        drop(state);

        let (data, poisoned) = unpack(self.data.write());
        let urw_guard = UrwLockWriteGuard {
            data, lock: self
        };
        pack(urw_guard, poisoned)
    }

    fn write_unlock(&self) {
        let mut state = self.state.lock().unwrap();
        state.writer = false;
        self.cond.notify_one();
    }
}

fn unpack<T>(result: LockResult<T>) -> (T, bool) {
    match result {
        Ok(inner) => (inner, false),
        Err(error) => (error.into_inner(), true)
    }
}

fn pack<T>(data: T, poisoned: bool) -> LockResult<T> {
    if poisoned {
        Err(PoisonError::new(data))
    } else {
        Ok(data)
    }
}