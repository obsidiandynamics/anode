use std::ops::Range;
use std::time::Duration;

/// A minimal specification of a 64-bit random number generator.
pub trait Rand64 {
    /// Return the next random `u64`.
    fn next_u64(&mut self) -> u64;
}

/// Randomly chooses a duration from a range.
pub trait RandDuration {
    fn gen_range(&mut self, range: Range<Duration>) -> Duration;
}

impl<R: Rand64> RandDuration for R {
    #[inline(always)]
    fn gen_range(&mut self, range: Range<Duration>) -> Duration {
        if range.is_empty() {
            return range.start;
        }
        let span = (range.end - range.start).as_nanos();
        let random = (self.next_u64() as u128) << 64 | (self.next_u64() as u128);
        let next = random % span;
        range.start + duration_from_nanos(next)
    }
}

const NANOS_PER_SEC: u128 = 1_000_000_000;

/// [`Duration::from_nanos`] has limited range, which [was not reverted post-stabilisation](https://github.com/rust-lang/rust/issues/51107).
/// This function permits the creation of a [`Duration]` from a `u128`, making it consistent with
/// [`Duration::as_nanos`].
#[inline(always)]
pub const fn duration_from_nanos(nanos: u128) -> Duration {
    let secs = (nanos / NANOS_PER_SEC) as u64;
    let nanos = (nanos % NANOS_PER_SEC) as u32;
    Duration::new(secs, nanos)
}

pub struct FixedDuration;

pub const FIXED_DURATION: FixedDuration = FixedDuration;

const NANOSECOND: Duration = Duration::new(0, 1);

impl Default for FixedDuration {
    #[inline(always)]
    fn default() -> Self {
        Self
    }
}

impl RandDuration for FixedDuration {
    #[inline(always)]
    fn gen_range(&mut self, range: Range<Duration>) -> Duration {
        if range.is_empty() {
            range.end
        } else {
            range.end - NANOSECOND
        }
    }
}

/// Basic [Xorshift](https://en.wikipedia.org/wiki/Xorshift) RNG.
pub struct Xorshift {
    seed: u64,
}

impl Xorshift {
    pub fn seed(seed: u64) -> Xorshift {
        Self { seed }
    }
}

impl Default for Xorshift {
    fn default() -> Self {
        Self::seed(1)
    }
}

impl Rand64 for Xorshift {
    fn next_u64(&mut self) -> u64 {
        let mut s = self.seed;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        self.seed = s;
        s
    }
}

#[cfg(test)]
mod tests;