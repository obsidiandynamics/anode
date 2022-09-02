use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use std::time::Duration;
use anode::zlock::UpgradeOutcome;

pub trait ReadGuardSpec<'a, T>: Deref<Target = T> {}

pub trait WriteGuardSpec<'a, T>: DerefMut<Target = T> {}

pub trait LockSpec<'a>: Sync + Send {
    type T: 'a;
    type R: ReadGuardSpec<'a, Self::T>;
    type W: WriteGuardSpec<'a, Self::T>;

    fn new(t: Self::T) -> Self;

    fn supports_read() -> bool;

    fn supports_downgrade() -> bool;

    fn supports_upgrade() -> bool;

    fn try_read(&'a self, duration: Duration) -> Option<Self::R>;

    fn try_write(&'a self, duration: Duration) -> Option<Self::W>;

    fn downgrade(guard: Self::W) -> Self::R;

    fn try_upgrade(guard: Self::R, duration: Duration) -> UpgradeOutcome<Self::W, Self::R>;
}

pub struct NoReadGuard<T> {
    __phantom_data: PhantomData<T>,
}

impl<T> Default for NoReadGuard<T> {
    fn default() -> Self {
        Self {
            __phantom_data: PhantomData::default(),
        }
    }
}

impl<T> Deref for NoReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unimplemented!()
    }
}

impl<'a, T> ReadGuardSpec<'a, T> for NoReadGuard<T> {}

#[cfg(test)]
mod tests;
