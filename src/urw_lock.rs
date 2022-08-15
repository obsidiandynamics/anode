use std::ops::{Deref, DerefMut};
use std::sync::{
    Condvar, LockResult, Mutex, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct InternalState {
    readers: u32,
    writer: bool,
}

pub struct UrwLock<T: ?Sized> {
    state: Mutex<InternalState>,
    cond: Condvar,
    data: RwLock<T>,
}

pub struct UrwLockReadGuard<'a, T> {
    data: Option<RwLockReadGuard<'a, T>>,
    lock: &'a UrwLock<T>,
}

impl<T> Drop for UrwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        if let Some(_) = self.data.take() {
            self.lock.read_unlock();
        }
    }
}

impl<'a, T> UrwLockReadGuard<'a, T> {
    pub fn upgrade(mut self) -> UrwLockWriteGuard<'a, T> {
        self.data.take();
        self.lock.upgrade()
    }
}

impl<T> Deref for UrwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data.as_ref().unwrap().deref()
    }
}

pub struct UrwLockWriteGuard<'a, T> {
    data: Option<RwLockWriteGuard<'a, T>>,
    lock: &'a UrwLock<T>,
}

impl<T> Drop for UrwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        if let Some(_) = self.data.take() {
            self.lock.write_unlock();
        }
    }
}

impl<'a, T> UrwLockWriteGuard<'a, T> {
    pub fn downgrade(mut self) -> UrwLockReadGuard<'a, T> {
        self.data.take();
        self.lock.downgrade()
    }
}

impl<T> Deref for UrwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data.as_ref().unwrap().deref()
    }
}

impl<T> DerefMut for UrwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data.as_mut().unwrap().deref_mut()
    }
}

enum Deadline {
    Point(Instant),
    Perpetual,
    Uninitialized(Duration),
}

impl Deadline {
    fn after(duration: Duration) -> Self {
        Self::Uninitialized(duration)
    }

    fn saturating_add(instant: Instant, duration: Duration) -> Self {
        match instant.checked_add(duration) {
            None => Deadline::Perpetual,
            Some(instant) => Deadline::Point(instant),
        }
    }

    fn ensure_initialized(&mut self) {
        if let Self::Uninitialized(duration) = self {
            *self = Self::saturating_add(Instant::now(), *duration);
        }
    }

    fn remaining(&mut self) -> Duration {
        self.ensure_initialized();

        match self {
            Deadline::Point(instant) => Instant::now() - *instant,
            Deadline::Perpetual => Duration::MAX,
            _ => unreachable!(),
        }
    }
}

impl<T> UrwLock<T> {
    pub fn new(t: T) -> Self {
        Self {
            state: Mutex::new(InternalState::default()),
            cond: Condvar::new(),
            data: RwLock::new(t),
        }
    }

    pub fn read(&self) -> LockResult<UrwLockReadGuard<'_, T>> {
        self.read_bounded(Duration::MAX).unwrap()
    }

    pub fn read_bounded(&self, duration: Duration) -> Option<LockResult<UrwLockReadGuard<'_, T>>> {
        let mut state = self.state.lock().unwrap();
        let mut deadline = Deadline::after(duration);
        while state.writer {
            let (guard, maybe_timed_out) =
                self.cond.wait_timeout(state, deadline.remaining()).unwrap();

            if maybe_timed_out.timed_out() {
                return None;
            }
            state = guard;
        }
        state.readers += 1;
        drop(state);

        let (data, poisoned) = unpack(self.data.read());
        let urw_guard = UrwLockReadGuard {
            data: Some(data),
            lock: self,
        };
        Some(pack(urw_guard, poisoned))
    }

    pub fn read_x(&self) -> LockResult<UrwLockReadGuard<'_, T>> {
        let mut state = self.state.lock().unwrap();
        while state.writer {
            state = self.cond.wait(state).unwrap();
        }
        state.readers += 1;
        drop(state);

        let (data, poisoned) = unpack(self.data.read());
        let urw_guard = UrwLockReadGuard {
            data: Some(data),
            lock: self,
        };
        pack(urw_guard, poisoned)
    }

    fn read_unlock(&self) {
        let mut state = self.state.lock().unwrap();
        assert!(state.readers > 0, "readers: {}", state.readers);
        state.readers -= 1;
        let readers = state.readers;
        drop(state);
        if readers == 1 {
            self.cond.notify_all();
        } else if readers == 0 {
            self.cond.notify_one();
        }
    }

    pub fn write(&self) -> LockResult<UrwLockWriteGuard<'_, T>> {
        self.write_bounded(Duration::MAX).unwrap()
    }

    pub fn write_bounded(
        &self,
        duration: Duration,
    ) -> Option<LockResult<UrwLockWriteGuard<'_, T>>> {
        let mut state = self.state.lock().unwrap();
        let mut deadline = Deadline::after(duration);
        while state.readers != 0 || state.writer {
            let (guard, maybe_timed_out) =
                self.cond.wait_timeout(state, deadline.remaining()).unwrap();

            if maybe_timed_out.timed_out() {
                return None;
            }
            state = guard;
        }
        state.writer = true;
        drop(state);

        let (data, poisoned) = unpack(self.data.write());
        let urw_guard = UrwLockWriteGuard {
            data: Some(data),
            lock: self,
        };
        Some(pack(urw_guard, poisoned))
    }

    pub fn write_x(
        &self,
    ) -> LockResult<UrwLockWriteGuard<'_, T>> {
        let mut state = self.state.lock().unwrap();
        while state.readers != 0 || state.writer {
            state = self.cond.wait(state).unwrap();
        }
        state.writer = true;
        drop(state);

        let (data, poisoned) = unpack(self.data.write());
        let urw_guard = UrwLockWriteGuard {
            data: Some(data),
            lock: self,
        };
        pack(urw_guard, poisoned)
    }

    fn write_unlock(&self) {
        let mut state = self.state.lock().unwrap();
        state.writer = false;
        drop(state);
        self.cond.notify_one();
    }

    fn downgrade(&self) -> UrwLockReadGuard<'_, T> {
        let mut state = self.state.lock().unwrap();
        state.readers = 1;
        state.writer = false;
        drop(state);
        self.cond.notify_all();
        let (data, _) = unpack(self.data.read());
        UrwLockReadGuard {
            data: Some(data),
            lock: self,
        }
    }

    fn upgrade(&self) -> UrwLockWriteGuard<'_, T> {
        let mut state = self.state.lock().unwrap();
        while state.readers != 1 {
            state = self.cond.wait(state).unwrap();
        }
        state.readers = 0;
        state.writer = true;
        drop(state);
        let (data, _) = unpack(self.data.write());
        UrwLockWriteGuard {
            data: Some(data),
            lock: self,
        }
    }
}

fn unpack<T>(result: LockResult<T>) -> (T, bool) {
    match result {
        Ok(inner) => (inner, false),
        Err(error) => (error.into_inner(), true),
    }
}

fn pack<T>(data: T, poisoned: bool) -> LockResult<T> {
    if poisoned {
        Err(PoisonError::new(data))
    } else {
        Ok(data)
    }
}
