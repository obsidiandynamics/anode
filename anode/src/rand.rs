use crate::inf_iterator::InfIterator;
use std::ops::Range;
use std::time::{Duration, SystemTime};

/// A minimal specification of a 64-bit random number generator.
pub trait Rand {
    /// Returns the next random `u64`.
    fn next_u64(&mut self) -> u64;

    /// Returns the next random `u128`.
    #[inline(always)]
    fn next_u128(&mut self) -> u128 {
        (self.next_u64() as u128) << 64 | (self.next_u64() as u128)
    }

    /// Returns a `bool` with a probability `p` of being true.
    ///
    /// # Example
    /// ```
    /// use anode::rand::{Probability, Rand, Xorshift};
    /// let mut rng = Xorshift::default();
    /// println!("{}", rng.next_bool(Probability::new(1.0 / 3.0)));
    /// ```
    #[inline(always)]
    fn next_bool(&mut self, p: Probability) -> bool {
        let cutoff = (p.0 * u64::MAX as f64) as u64;
        let mut next = self.next_u64();
        if next == u64::MAX {
            // guarantees that gen_bool(p=1.0) is never true
            next = u64::MAX - 1;
        }
        next < cutoff
    }
}

/// Represents a probability in the range \[0, 1\].
#[derive(Clone, Copy)]
pub struct Probability(f64);

impl Probability {
    /// Creates a new [`Probability`] value, bounded in the range \[0, 1\].
    ///
    /// # Example
    /// ```
    /// use anode::rand::Probability;
    /// let p = Probability::new(0.25);
    /// assert_eq!(0.25, p.into());
    /// ```
    ///
    /// # Panics
    /// If `p < 0` or `p > 1`.
    #[inline(always)]
    pub fn new(p: f64) -> Self {
        assert!(p >= 0f64, "p ({p}) cannot be less than 0");
        assert!(p <= 1f64, "p ({p}) cannot be greater than 1");
        Self(p)
    }

    /// Creates a new [`Probability`] value, without checking the bounds. If a
    /// probability is created outside the range \[0, 1\], its behaviour with an
    /// RNG is undefined.
    #[inline(always)]
    pub const unsafe fn new_unchecked(p: f64) -> Self {
        Self(p)
    }
}

impl From<Probability> for f64 {
    #[inline(always)]
    fn from(p: Probability) -> Self {
        p.0
    }
}

impl From<f64> for Probability {
    #[inline(always)]
    fn from(p: f64) -> Self {
        Probability::new(p)
    }
}

/// The means for seeding an RNG.
pub trait Seeded {
    type Rng: Rand;

    /// Creates a new [`Rand64`] instance from the given seed.
    fn seed(seed: u64) -> Self::Rng;
}

pub trait RandLim<N> {
    /// Generates a random number in `0..N`.
    fn next_lim(&mut self, lim: N) -> N;
}

impl<R: Rand> RandLim<u64> for R {
    #[inline(always)]
    fn next_lim(&mut self, lim: u64) -> u64 {
        let mut full = self.next_u64() as u128 * lim as u128;
        let mut low = full as u64;
        if low < lim {
            let cutoff = lim.wrapping_neg() % lim;
            while low < cutoff {
                full = self.next_u64() as u128 * lim as u128;
                low = full as u64;
            }
        }
        (full >> 64) as u64
    }
}

impl<R: Rand> RandLim<u128> for R {
    #[inline(always)]
    fn next_lim(&mut self, lim: u128) -> u128 {
        if lim <= u64::MAX as u128 {
            self.next_lim(lim as u64) as u128
        } else {
            let cutoff = cutoff_u128(lim);
            loop {
                let rand = self.next_u128();
                if rand <= cutoff {
                    return rand % lim;
                }
            }
        }
    }
}

// #[inline(always)]
// fn cutoff_u64(lim: u64) -> u64 {
//     let overhang = (u64::MAX - lim + 1) % lim;
//     u64::MAX - overhang
// }

#[inline(always)]
fn cutoff_u128(lim: u128) -> u128 {
    let overhang = (u128::MAX - lim + 1) % lim;
    u128::MAX - overhang
}

pub trait RandRange<N> {
    /// Generates a random number in the given range.
    fn next_range(&mut self, range: Range<N>) -> N;
}

impl<R: Rand> RandRange<u64> for R {
    #[inline(always)]
    fn next_range(&mut self, range: Range<u64>) -> u64 {
        if range.is_empty() {
            return range.start;
        }
        let span = range.end - range.start;
        range.start + self.next_lim(span)
    }
}

impl<R: Rand> RandRange<u128> for R {
    #[inline(always)]
    fn next_range(&mut self, range: Range<u128>) -> u128 {
        if range.is_empty() {
            return range.start;
        }
        let span = range.end - range.start;
        let random = self.next_lim(span);
        range.start + random
    }
}

impl<R: Rand> RandRange<Duration> for R {
    #[inline(always)]
    fn next_range(&mut self, range: Range<Duration>) -> Duration {
        if range.is_empty() {
            return range.start;
        }
        let span = (range.end - range.start).as_nanos();
        let random = self.next_lim(span);
        range.start + duration_from_nanos(random)
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

impl RandRange<Duration> for FixedDuration {
    #[inline(always)]
    fn next_range(&mut self, range: Range<Duration>) -> Duration {
        const NANOSECOND: Duration = Duration::new(0, 1);
        if range.is_empty() {
            range.start
        } else {
            range.end - NANOSECOND
        }
    }
}

/// Basic [Xorshift](https://en.wikipedia.org/wiki/Xorshift) RNG.
pub struct Xorshift(u64);

impl Default for Xorshift {
    #[inline(always)]
    fn default() -> Self {
        Self(1)
    }
}

impl Rand for Xorshift {
    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        let mut s = self.0;
        s ^= s << 13;
        s ^= s >> 7;
        self.0 = s;
        s ^= s << 17;
        s
    }
}

impl Seeded for Xorshift {
    type Rng = Xorshift;

    #[inline(always)]
    fn seed(seed: u64) -> Self::Rng {
        // a zero seed disables Xorshift, rendering it (effectively) a constant; hence, we avoid it
        Self(if seed == 0 { u64::MAX >> 1 } else { seed })
    }
}

pub struct Wyrand(u64);

impl Default for Wyrand {
    #[inline(always)]
    fn default() -> Self {
        Self(0)
    }
}

impl Rand for Wyrand {
    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0xA0761D6478BD642F);
        let r = self.0 as u128 * (self.0 ^ 0xE7037ED1A0B428DB) as u128;
        (r as u64) ^ (r >> 64) as u64
    }
}

impl Seeded for Wyrand {
    type Rng = Wyrand;

    #[inline(always)]
    fn seed(seed: u64) -> Self::Rng {
        Self(seed)
    }
}

#[derive(Debug)]
pub struct CyclicSeed(u64);

impl CyclicSeed {
    #[inline]
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
}

impl Default for CyclicSeed {
    #[inline]
    fn default() -> Self {
        Self(0)
    }
}

impl InfIterator for CyclicSeed {
    type Item = u64;

    #[inline]
    fn next(&mut self) -> Self::Item {
        let current = self.0;
        self.0 = if current == u64::MAX { 0 } else { current + 1 };
        current
    }
}

pub struct LazyRand64<S: Seeded, F: FnOnce() -> u64> {
    state: Option<InitState<S::Rng, F>>,
}

impl<S: Seeded, F: FnOnce() -> u64> LazyRand64<S, F> {
    pub fn lazy(f: F) -> Self {
        Self {
            state: Some(InitState::Uninit(f)),
        }
    }

    pub fn eager(rng: S::Rng) -> Self {
        Self {
            state: Some(InitState::Ready(rng)),
        }
    }
}

enum InitState<R: Rand, F: FnOnce() -> u64> {
    Uninit(F),
    Ready(R),
}

impl<S: Seeded, F: FnOnce() -> u64> Rand for LazyRand64<S, F> {
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

/// Derives a seed from the system clock by XORing the upper 64 bits of the nanosecond timestamp
/// with the lower 64 bits.
pub fn clock_seed() -> u64 {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let folded = (time >> 64) ^ time;
    folded as u64
}

#[cfg(test)]
mod tests;
