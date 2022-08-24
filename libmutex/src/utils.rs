use std::sync::{Condvar, LockResult, MutexGuard, TryLockError, TryLockResult};
use std::time::{Duration};

// pub fn unpack<T>(result: LockResult<T>) -> (T, bool) {
//     match result {
//         Ok(inner) => (inner, false),
//         Err(error) => (error.into_inner(), true),
//     }
// }

/// From _Poison_, by _The Prodigy_ (1994).
/// I got the poison,
/// I got the **remedy**...
///
/// This function unpacks a [`LockResult`], extracting the locked data.
/// It doesn't care if the data has been poisoned -- presumably, the caller
/// already has a way of dealing with this.
// #[inline(always)]
// pub fn remedy<T>(result: LockResult<T>) -> T {
//     match result {
//         Ok(inner) => inner,
//         Err(error) => error.into_inner(),
//     }
// }
//
// /// A variant of [`remedy`] for a [`TryLockResult`].
// #[inline(always)]
// pub fn try_remedy<T>(result: TryLockResult<T>) -> Option<T> {
//     match result {
//         Ok(inner) => Some(inner),
//         Err(TryLockError::Poisoned(error)) => Some(error.into_inner()),
//         Err(TryLockError::WouldBlock) => None,
//     }
// }

/// From _Poison_, by _The Prodigy_ (1994).
/// I got the poison,
/// I got the **remedy**...
///
/// Unpacks a lock result, extracting the locked data.
/// It doesn't care if the data has been poisoned -- presumably, the caller
/// already has a way of dealing with this.
pub trait Remedy<T> {
    type Output;

    fn remedy(self) -> Self::Output;
}

impl<T> Remedy<T> for LockResult<T> {
    type Output = T;

    #[inline(always)]
    fn remedy(self) -> Self::Output {
        match self {
            Ok(inner) => inner,
            Err(error) => error.into_inner(),
        }
    }
}

impl<T> Remedy<T> for TryLockResult<T> {
    type Output = Option<T>;

    #[inline(always)]
    fn remedy(self) -> Self::Output {
        match self {
            Ok(inner) => Some(inner),
            Err(TryLockError::Poisoned(error)) => Some(error.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        }
    }
}

// #[inline]
// pub fn pack<T>(data: T, poisoned: bool) -> LockResult<T> {
//     if poisoned {
//         Err(PoisonError::new(data))
//     } else {
//         Ok(data)
//     }
// }
//
// #[inline]
// fn cond_wait<'a, T>(
//     cond: &Condvar,
//     guard: MutexGuard<'a, T>,
//     duration: Duration,
// ) -> LockResult<(MutexGuard<'a, T>, bool)> {
//     if duration.is_zero() {
//         pack((guard, true), false)
//     } else if duration == Duration::MAX {
//         let (guard, poisoned) = unpack(cond.wait(guard));
//         pack((guard, false), poisoned)
//     } else {
//         let ((guard, maybe_timed_out), poisoned) = unpack(cond.wait_timeout(guard, duration));
//         pack((guard, maybe_timed_out.timed_out()), poisoned)
//     }
// }

#[inline(always)]
pub fn cond_wait_remedy<'a, T>(
    cond: &Condvar,
    guard: MutexGuard<'a, T>,
    duration: Duration,
) -> (MutexGuard<'a, T>, bool) {
    if duration.is_zero() {
        (guard, true)
    } else if duration == Duration::MAX {
        let guard = cond.wait(guard).remedy();
        (guard, false)
    } else {
        let (guard, maybe_timed_out) = cond.wait_timeout(guard, duration).remedy();
        (guard, maybe_timed_out.timed_out())
    }
}