use std::marker::PhantomData;
use std::ops::Range;
use std::time::{Duration, SystemTime};

/// A minimal specification of a 64-bit random number generator.
pub trait Rand64 {
    /// Return the next random `u64`.
    fn next_u64(&mut self) -> u64;
}

/// The means for seeding an RNG.
pub trait Seeded {
    type Rng: Rand64;

    /// Creates a new [`Rand64`] instance from the given seed.
    fn seed(seed: u64) -> Self::Rng;
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
        if span <= u64::MAX as u128 {
            let span = span as u64;
            let random = span.gen(self);
            range.start + Duration::from_nanos(random)
        } else {
            let random = span.gen(self);
            range.start + duration_from_nanos(random)
        }
    }
}

trait GenSpan {
    fn gen(&self, rng: &mut impl Rand64) -> Self;
}

impl GenSpan for u64 {
    #[inline(always)]
    fn gen(&self, rng: &mut impl Rand64) -> Self {
        rng.next_u64() % self
    }
}

impl GenSpan for u128 {
    #[inline(always)]
    fn gen(&self, rng: &mut impl Rand64) -> Self {
        let random = (rng.next_u64() as u128) << 64 | (rng.next_u64() as u128);
        random % self
    }
}

/// [`Duration::from_nanos`] has limited range, which [was not reverted post-stabilisation](https://github.com/rust-lang/rust/issues/51107).
/// This function permits the creation of a [`Duration]` from a `u128`, making it consistent with
/// [`Duration::as_nanos`].
#[inline(always)]
pub const fn duration_from_nanos(nanos: u128) -> Duration {
    const NANOS_PER_SEC: u128 = 1_000_000_000;
    let secs = (nanos / NANOS_PER_SEC) as u64;
    let nanos = (nanos % NANOS_PER_SEC) as u32;
    Duration::new(secs, nanos)
}

pub struct FixedDuration;

pub const FIXED_DURATION: FixedDuration = FixedDuration;

impl Default for FixedDuration {
    #[inline(always)]
    fn default() -> Self {
        Self
    }
}

impl RandDuration for FixedDuration {
    #[inline(always)]
    fn gen_range(&mut self, range: Range<Duration>) -> Duration {
        const NANOSECOND: Duration = Duration::new(0, 1);
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

impl Seeded for Xorshift {
    type Rng = Xorshift;

    fn seed(seed: u64) -> Self::Rng {
        Self { seed }
    }
}

pub struct LazyRand64<S: Seeded, F: FnOnce() -> u64> {
    state: Option<InitState<S::Rng, F>>,
    __phantom_data: PhantomData<S>,
}

impl<S: Seeded, F: FnOnce() -> u64> LazyRand64<S, F> {
    pub fn lazy(f: F) -> Self {
        Self {
            state: Some(InitState::Uninit(f)),
            __phantom_data: PhantomData::default(),
        }
    }

    pub fn eager(rng: S::Rng) -> Self {
        Self {
            state: Some(InitState::Ready(rng)),
            __phantom_data: PhantomData::default(),
        }
    }
}

enum InitState<R: Rand64, F: FnOnce() -> u64> {
    Uninit(F),
    Ready(R),
}

impl<S: Seeded, F: FnOnce() -> u64> Rand64 for LazyRand64<S, F> {
    fn next_u64(&mut self) -> u64 {
        match self.state.take() {
            Some(InitState::Uninit(f)) => {
                let seed = f();
                let mut rng = S::seed(seed);
                let next = rng.next_u64();
                self.state = Some(InitState::<S::Rng, F>::Ready(rng));
                next
            }
            Some(InitState::Ready(mut rng)) => {
                let next = rng.next_u64();
                self.state = Some(InitState::Ready(rng));
                next
            }
            None => unreachable!(),
        }
    }
}

pub fn clock_seed() -> u64 {
    let time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    time as u64
}

#[cfg(test)]
mod tests;
