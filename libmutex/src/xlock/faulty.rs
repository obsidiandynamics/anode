use std::time::Duration;
use crate::xlock::Moderator;

#[derive(Debug)]
pub struct Faulty;

#[derive(Debug)]
pub struct FaultySync {
}

impl Moderator for Faulty {
    type Sync = FaultySync;

    #[inline]
    fn new() -> Self::Sync {
        Self::Sync {}
    }

    #[inline]
    fn try_read(_sync: &Self::Sync, _duration: Duration) -> bool {
        true
    }

    #[inline]
    fn read_unlock(_sync: &Self::Sync) {
    }

    #[inline]
    fn try_write(_sync: &Self::Sync, _duration: Duration) -> bool {
        true
    }

    #[inline]
    fn write_unlock(_sync: &Self::Sync) {
    }

    fn downgrade(_sync: &Self::Sync) {
    }

    fn try_upgrade(_sync: &Self::Sync, _duration: Duration) -> bool {
        true
    }
}