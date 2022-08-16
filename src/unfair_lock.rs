use std::ops::{Deref, DerefMut};
use std::sync::{
    Condvar, LockResult, Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct InternalState {
    readers: u32,
    writer: bool,
}

pub struct UnfairLock<T: ?Sized> {
    state: Mutex<InternalState>,
    cond: Condvar,
    data: RwLock<T>,
}

pub struct LockReadGuard<'a, T> {
    data: Option<RwLockReadGuard<'a, T>>,
    lock: &'a UnfairLock<T>,
}

impl<T> Drop for LockReadGuard<'_, T> {
    fn drop(&mut self) {
        if let Some(_) = self.data.take() {
            self.lock.read_unlock();
        }
    }
}

impl<'a, T> LockReadGuard<'a, T> {
    pub fn upgrade(mut self) -> LockWriteGuard<'a, T> {
        self.data.take();
        self.lock.upgrade()
    }
}

impl<T> Deref for LockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data.as_ref().unwrap().deref()
    }
}

pub struct LockWriteGuard<'a, T> {
    data: Option<RwLockWriteGuard<'a, T>>,
    lock: &'a UnfairLock<T>,
}

impl<T> Drop for LockWriteGuard<'_, T> {
    fn drop(&mut self) {
        if let Some(_) = self.data.take() {
            self.lock.write_unlock();
        }
    }
}

impl<'a, T> LockWriteGuard<'a, T> {
    pub fn downgrade(mut self) -> LockReadGuard<'a, T> {
        self.data.take();
        self.lock.downgrade()
    }
}

impl<T> Deref for LockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data.as_ref().unwrap().deref()
    }
}

impl<T> DerefMut for LockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data.as_mut().unwrap().deref_mut()
    }
}

enum Deadline {
    Point(Instant),
    Perpetual,
    Uninitialized(Duration),
    Elapsed,
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
            if duration == &Duration::MAX {
                *self = Deadline::Perpetual;
            } else if duration ==  &Duration::ZERO {
                *self = Deadline::Elapsed;
            } else {
                *self = Self::saturating_add(Instant::now(), *duration);
            }
        }
    }

    fn remaining(&mut self) -> Duration {
        self.ensure_initialized();

        match self {
            Deadline::Point(instant) => Instant::now() - *instant,
            Deadline::Perpetual => Duration::MAX,
            Deadline::Elapsed => Duration::ZERO,
            _ => unreachable!(),
        }
    }
}

impl<T> UnfairLock<T> {
    pub fn new(t: T) -> Self {
        Self {
            state: Mutex::new(InternalState::default()),
            cond: Condvar::new(),
            data: RwLock::new(t),
        }
    }

    pub fn read(&self) -> LockReadGuard<'_, T> {
        self.try_read(Duration::MAX).unwrap()
    }

    pub fn try_read(&self, duration: Duration) -> Option<LockReadGuard<'_, T>> {
        let mut state = self.state.lock().unwrap();
        let mut deadline = Deadline::after(duration);
        while state.writer {
            let (guard, timed_out) =
                cond_wait(&self.cond, state, deadline.remaining()).unwrap();

            if timed_out {
                return None;
            }
            state = guard;
        }
        state.readers += 1;
        drop(state);

        let (data, _) = unpack(self.data.read());
        let read_guard = LockReadGuard {
            data: Some(data),
            lock: self,
        };
        Some(read_guard)
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

    pub fn write(&self) -> LockWriteGuard<'_, T> {
        self.try_write(Duration::MAX).unwrap()
    }

    pub fn try_write(
        &self,
        duration: Duration,
    ) -> Option<LockWriteGuard<'_, T>> {
        let mut state = self.state.lock().unwrap();
        let mut deadline = Deadline::after(duration);
        while state.readers != 0 || state.writer {
            let (guard, timed_out) =
                cond_wait(&self.cond, state, deadline.remaining()).unwrap();

            if timed_out {
                return None;
            }
            state = guard;
        }
        state.writer = true;
        drop(state);

        let (data, _) = unpack(self.data.write());
        let write_guard = LockWriteGuard {
            data: Some(data),
            lock: self,
        };
        Some(write_guard)
    }

    fn write_unlock(&self) {
        let mut state = self.state.lock().unwrap();
        state.writer = false;
        drop(state);
        self.cond.notify_one();
    }

    fn downgrade(&self) -> LockReadGuard<'_, T> {
        let mut state = self.state.lock().unwrap();
        state.readers = 1;
        state.writer = false;
        drop(state);
        self.cond.notify_all();
        let (data, _) = unpack(self.data.read());
        LockReadGuard {
            data: Some(data),
            lock: self,
        }
    }

    fn upgrade(&self) -> LockWriteGuard<'_, T> {
        let mut state = self.state.lock().unwrap();
        while state.readers != 1 {
            state = self.cond.wait(state).unwrap();
        }
        state.readers = 0;
        state.writer = true;
        drop(state);
        let (data, _) = unpack(self.data.write());
        LockWriteGuard {
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

fn cond_wait<'a, T>(
    cond: &Condvar,
    guard: MutexGuard<'a, T>,
    duration: Duration,
) -> LockResult<(MutexGuard<'a, T>, bool)> {
    if duration == Duration::MAX {
        let (guard, poisoned) = unpack(cond.wait(guard));
        pack((guard, false), poisoned)
    } else {
        let ((guard, maybe_timed_out), poisoned) = unpack(cond.wait_timeout(guard, duration));
        pack((guard, maybe_timed_out.timed_out()), poisoned)
    }
}
