use std::ops::Range;
use std::time::{Duration};
use rand::{thread_rng};
use libmutex::rand::{Rand64, Xorshift};

fn main() {
    let mut rng = thread_rng();
    println!("from zero");
    println!("random: {:?}", gen_range(&mut rng, Duration::ZERO..Duration::ZERO));
    println!("random: {:?}", gen_range(&mut rng, Duration::ZERO..Duration::from_nanos(100)));
    println!("random: {:?}", gen_range(&mut rng, Duration::ZERO..Duration::from_micros(100)));
    println!("random: {:?}", gen_range(&mut rng, Duration::ZERO..Duration::from_millis(100)));
    println!("random: {:?}", gen_range(&mut rng, Duration::ZERO..Duration::from_secs(100)));
    println!("random: {:?}", gen_range(&mut rng, Duration::ZERO..Duration::MAX));

    println!("from half");
    println!("random: {:?}", gen_range(&mut rng, Duration::from_nanos(50)..Duration::from_nanos(100)));
    println!("random: {:?}", gen_range(&mut rng, Duration::from_micros(50)..Duration::from_micros(100)));
    println!("random: {:?}", gen_range(&mut rng, Duration::from_millis(50)..Duration::from_millis(100)));
    println!("random: {:?}", gen_range(&mut rng, Duration::from_secs(50)..Duration::from_secs(100)));
    println!("random: {:?}", gen_range(&mut rng, Duration::from_secs(u64::MAX >> 2)..Duration::MAX));

    println!("from top");
    println!("random: {:?}", gen_range(&mut rng, Duration::from_nanos(100)..Duration::from_nanos(100)));
    println!("random: {:?}", gen_range(&mut rng, Duration::from_micros(100)..Duration::from_micros(100)));
    println!("random: {:?}", gen_range(&mut rng, Duration::from_millis(100)..Duration::from_millis(100)));
    println!("random: {:?}", gen_range(&mut rng, Duration::from_secs(100)..Duration::from_secs(100)));
    println!("random: {:?}", gen_range(&mut rng, Duration::MAX..Duration::MAX));

    println!("excess");
    println!("random: {:?}", gen_range(&mut rng, Duration::from_secs(101)..Duration::from_secs(100)));

    let mut x = Xorshift::seed(1);
    for _ in 0..100 {
        println!("next: {}", x.next_u64());
    }
}

fn gen_range(rng: &mut impl Rand64, range: Range<Duration>) -> Duration {
    if range.is_empty() {
        return range.start;
    }
    let span = (range.end - range.start).as_nanos() - 1;
    let random = (rng.next_u64() as u128) << 64 | (rng.next_u64() as u128);
    let next = random % span;
    range.start + from_nanos(next)
}

const NANOS_PER_SEC: u128 = 1_000_000_000;

pub const fn from_nanos(nanos: u128) -> Duration {
    let secs = (nanos / NANOS_PER_SEC) as u64;
    let nanos = (nanos % NANOS_PER_SEC) as u32;
    Duration::new(secs, nanos)
}