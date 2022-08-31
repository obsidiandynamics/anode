use super::*;
use std::time::Duration;

#[test]
fn duration_from_nanos_reversible() {
    let cases = vec![
        Duration::ZERO,
        Duration::from_nanos(1),
        Duration::from_micros(1),
        Duration::from_millis(1),
        Duration::from_secs(1),
        Duration::MAX,
    ];

    for case in cases {
        let nanos = case.as_nanos();
        let duration = duration_from_nanos(nanos);
        assert_eq!(case, duration);
    }
}

#[test]
fn fixed_duration() {
    assert_eq!(
        Duration::ZERO,
        FixedDuration::default().gen_range(Duration::ZERO..Duration::ZERO)
    );
    assert_eq!(
        Duration::ZERO,
        FixedDuration::default().gen_range(Duration::ZERO..Duration::from_nanos(1))
    );
    assert_eq!(
        Duration::from_nanos(1),
        FixedDuration::default().gen_range(Duration::ZERO..Duration::from_nanos(2))
    );
    assert_eq!(
        Duration::MAX - Duration::from_nanos(1),
        FixedDuration::default().gen_range(Duration::ZERO..Duration::MAX)
    );
}

#[test]
fn random_duration() {
    const NANOSECOND: Duration = Duration::new(0, 1);
    let mut rng = Xorshift::default();

    #[derive(Debug)]
    struct Case {
        range: Range<Duration>,
        exp_min: Duration,
        exp_max: Duration,
    }
    for case in &vec![
        // from zero
        Case {
            range: Duration::ZERO..Duration::ZERO,
            exp_min: Duration::ZERO,
            exp_max: Duration::ZERO,
        },
        Case {
            range: Duration::ZERO..Duration::from_nanos(100),
            exp_min: Duration::ZERO,
            exp_max: Duration::from_nanos(100) - NANOSECOND,
        },
        Case {
            range: Duration::ZERO..Duration::from_nanos(1),
            exp_min: Duration::ZERO,
            exp_max: Duration::ZERO,
        },
        Case {
            range: Duration::ZERO..Duration::from_micros(100),
            exp_min: Duration::ZERO,
            exp_max: Duration::from_micros(100) - NANOSECOND
        },
        Case {
            range: Duration::ZERO..Duration::from_millis(100),
            exp_min: Duration::ZERO,
            exp_max: Duration::from_millis(100) - NANOSECOND
        },
        Case {
            range: Duration::ZERO..Duration::from_secs(100),
            exp_min: Duration::ZERO,
            exp_max: Duration::from_secs(100) - NANOSECOND
        },
        Case {
            range: Duration::ZERO..Duration::MAX,
            exp_min: Duration::ZERO,
            exp_max: Duration::MAX - NANOSECOND
        },
        // from half
        Case {
            range: Duration::from_nanos(50)..Duration::from_nanos(100),
            exp_min: Duration::from_nanos(50),
            exp_max: Duration::from_nanos(100) - NANOSECOND
        },
        Case {
            range: Duration::from_nanos(50)..Duration::from_nanos(51),
            exp_min: Duration::from_nanos(50),
            exp_max: Duration::from_nanos(51) - NANOSECOND
        },
        Case {
            range: Duration::from_micros(50)..Duration::from_micros(100),
            exp_min: Duration::from_micros(50),
            exp_max: Duration::from_micros(100) - NANOSECOND
        },
        Case {
            range: Duration::from_millis(50)..Duration::from_millis(100),
            exp_min: Duration::from_millis(50),
            exp_max: Duration::from_millis(100) - NANOSECOND
        },
        Case {
            range: Duration::from_secs(50)..Duration::from_secs(100),
            exp_min: Duration::from_secs(50),
            exp_max: Duration::from_secs(100) - NANOSECOND
        },
        Case {
            range: Duration::from_secs(u64::MAX >> 1)..Duration::MAX,
            exp_min: Duration::from_secs(u64::MAX >> 1),
            exp_max: Duration::MAX - NANOSECOND
        }, 
        // from top
        Case {
            range: Duration::from_nanos(100)..Duration::from_nanos(100),
            exp_min: Duration::from_nanos(100),
            exp_max: Duration::from_nanos(100)
        },
        Case {
            range: Duration::from_micros(100)..Duration::from_micros(100),
            exp_min: Duration::from_micros(100),
            exp_max: Duration::from_micros(100)
        },
        Case {
            range: Duration::from_millis(100)..Duration::from_millis(100),
            exp_min: Duration::from_millis(100),
            exp_max: Duration::from_millis(100)
        },
        Case {
            range: Duration::from_secs(100)..Duration::from_secs(100),
            exp_min: Duration::from_secs(100),
            exp_max: Duration::from_secs(100)
        },
        // excess
        Case {
            range: Duration::from_secs(101)..Duration::from_secs(100),
            exp_min: Duration::from_secs(101),
            exp_max: Duration::from_secs(101)
        }
    ] {
        let d = rng.gen_range(case.range.clone());
        assert!(
            d >= case.exp_min,
            "for case {case:?} random duration was {d:?}"
        );
        assert!(
            d <= case.exp_max,
            "for case {case:?} random duration was {d:?}"
        );
    }
}

#[test]
fn xorshift_constant_with_zero_seed() {
    // initialise with 0 seed for testing only; the user cannot initialise Xorshift with 0
    let mut rng = Xorshift { seed: 0 };
    assert_eq!(0, rng.next_u64());
    assert_eq!(0, rng.next_u64());
}

#[test]
fn xorshift_seed() {
    let mut rng = Xorshift::seed(0); // should not initialise from 0 seed under the hood
    assert_eq!(u64::MAX >> 1, rng.seed);
    assert_ne!(0, rng.next_u64());

    let mut rng = Xorshift::seed(1); // every nonzero seed is okay
    assert_eq!(1, rng.seed);
    assert_ne!(0, rng.next_u64());

    let mut rng = Xorshift::seed(u64::MAX); // every nonzero seed is okay
    assert_eq!(u64::MAX, rng.seed);
    assert_ne!(0, rng.next_u64());
}

#[test]
fn cyclic_seed() {
    let mut seed = CyclicSeed::default();
    assert_eq!(0, seed.next());
    assert_eq!(1, seed.next());
    assert_eq!(2, seed.next());
    assert_eq!(3, seed.next());

    let mut seed = CyclicSeed::new(u64::MAX - 3);
    assert_eq!(u64::MAX - 3, seed.next());
    assert_eq!(u64::MAX - 2, seed.next());
    assert_eq!(u64::MAX - 1, seed.next());
    assert_eq!(u64::MAX, seed.next());
    assert_eq!(0, seed.next());
    assert_eq!(1, seed.next());
}

#[derive(Default, Debug)]
struct MockRng {
    next: u64,
}

impl Rand64 for MockRng {
    fn next_u64(&mut self) -> u64 {
        self.next
    }
}

#[test]
fn gen_bool() {
    // NB: no matter what the random number, p(0.0) should always evaluate to false,
    // while p(1.0) should always evaluate to true

    let mut rng = MockRng::default();
    rng.next = 0;
    assert!(!rng.gen_bool(0.0.into()));
    assert!(rng.gen_bool(f64::EPSILON.into()));
    assert!(rng.gen_bool(0.5.into()));
    assert!(rng.gen_bool(1.0.into()));

    rng.next = u64::MAX / 4;
    assert!(!rng.gen_bool(0.0.into()));
    assert!(!rng.gen_bool((0.25 - f64::EPSILON).into()));
    assert!(rng.gen_bool((0.25 + f64::EPSILON).into()));
    assert!(rng.gen_bool(1.0.into()));

    rng.next = u64::MAX / 2;
    assert!(!rng.gen_bool(0.0.into()));
    assert!(!rng.gen_bool((0.5 - f64::EPSILON).into()));
    assert!(rng.gen_bool((0.5 + f64::EPSILON).into()));
    assert!(rng.gen_bool(1.0.into()));

    rng.next = u64::MAX;
    assert!(!rng.gen_bool(0.0.into()));
    assert!(!rng.gen_bool(0.5.into()));
    assert!(!rng.gen_bool((1.0 - f64::EPSILON).into()));
    assert!(rng.gen_bool(1.0.into()));
}

#[test]
#[should_panic(expected="cannot be less than 0")]
fn probability_panics_lt_0() {
    Probability::new(0f64 - f64::EPSILON);
}

#[test]
#[should_panic(expected="cannot be greater than 1")]
fn probability_panics_gt_1() {
    Probability::new(1f64 + f64::EPSILON);
}