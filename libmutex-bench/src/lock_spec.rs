use std::ops::{Deref, DerefMut};
use std::time::Duration;
use libmutex::xlock::{LockReadGuard, LockWriteGuard, Moderator, UpgradeOutcome, XLock};

pub trait ReadGuardSpec<'a, T>: Deref<Target = T> {}

pub trait WriteGuardSpec<'a, T>: DerefMut<Target = T> {}

impl<'a, T, M: Moderator> ReadGuardSpec<'a, T> for LockReadGuard<'a, T, M> {}

impl<'a, T, M: Moderator> WriteGuardSpec<'a, T> for LockWriteGuard<'a, T, M> {}

pub trait LockSpec<'a>: Sync + Send {
    type T: 'a;
    type R: ReadGuardSpec<'a, Self::T>;
    type W: WriteGuardSpec<'a, Self::T>;

    fn new(t: Self::T) -> Self;

    fn try_read(&'a self, duration: Duration) -> Option<Self::R>;

    fn try_write(&'a self, duration: Duration) -> Option<Self::W>;

    fn downgrade(guard: Self::W) -> Self::R;

    fn try_upgrade(guard: Self::R, duration: Duration) -> UpgradeOutcome<Self::W, Self::R>;
}

impl<'a, T: Sync + Send + 'a, M: Moderator + 'a> LockSpec<'a> for XLock<T, M> {
    type T = T;
    type R = LockReadGuard<'a, T, M>;
    type W = LockWriteGuard<'a, T, M>;

    fn new(t: Self::T) -> Self {
        XLock::<_, M>::new(t)
    }

    fn try_read(&'a self, duration: Duration) -> Option<Self::R> {
        self.try_read(duration)
    }

    fn try_write(&'a self, duration: Duration) -> Option<Self::W> {
        self.try_write(duration)
    }

    fn downgrade(guard: Self::W) -> Self::R {
        guard.downgrade()
    }

    fn try_upgrade(guard: Self::R, duration: Duration) -> UpgradeOutcome<Self::W, Self::R> {
        guard.try_upgrade(duration)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    use libmutex::xlock::{ReadBiased, XLock};
    use crate::lock_spec::LockSpec;

    #[test]
    fn conformance() {
        let lock = XLock::<_, ReadBiased>::new(0);
        takes_borrowed(&lock);

        takes_owned(lock);

        takes_owned_alt(XLock::<_, ReadBiased>::new(0));
    }

    fn takes_borrowed<'a, L: LockSpec<'a, T=u64>>(lock: &'a L) {
        let guard = lock.try_read(Duration::ZERO).unwrap();
        assert_eq!(0, *guard);

        let mut guard = L::try_upgrade(guard, Duration::ZERO).upgraded().unwrap();
        assert_eq!(0, *guard);
        *guard = 42;

        let guard = L::downgrade(guard);
        assert_eq!(42, *guard);

        drop(guard);

        let mut guard = lock.try_write(Duration::ZERO).unwrap();
        assert_eq!(42, *guard);
        *guard = 69;

        let guard = L::downgrade(guard);
        assert_eq!(69, *guard);
    }

    fn takes_owned<L>(lock: L)
        where for <'a> L: LockSpec<'a, T=u64> + 'static
    {
        let arc = Arc::new(lock);
        thread::spawn(move || {
            arc.try_read(Duration::ZERO);
        }).join().unwrap();
    }

    fn takes_owned_alt<L: for <'a> LockSpec<'a, T=u64> + 'static>(lock: L) {
        let arc = Arc::new(lock);
        thread::spawn(move || {
            arc.try_read(Duration::ZERO);
        }).join().unwrap();
    }
}