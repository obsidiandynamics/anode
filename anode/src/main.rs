use std::env;
use std::io::{stdout, Write};
use std::process::exit;
use std::str::FromStr;
use anode::rand::{clock_seed, Probability, Rand64, Seeded, Wyrand, Xorshift};

fn main() {
    //TODO
    // let tries = 10000;
    // let p = Probability::new(0.05);
    // let mut rng = Xorshift::seed(clock_seed());
    // let mut was_true = 0;
    // for _ in 0..tries {
    //     let b = rng.next_bool(p);
    //     if b {
    //         was_true += 1;
    //     }
    // }
    // let rate = was_true as f64 / tries as f64;
    // println!("rate of true {}", rate);
    //
    // let n = 13u64;
    // let m = (u64::MAX - n) % n;
    // dbg!((n, m));

    let mut w = Wyrand::default();
    // for _ in 0..100 {
    //     println!("next = {}", w.next_u64());
    // }
    // let mut out = stdout();
    // let mut buf = [0u8; 1024];
    // loop {
    //     let mut i = 0;
    //     while i < 1024 {
    //         let rand = w.next_u64();
    //         buf[i] = rand as u8;
    //         buf[i + 1] = (rand >> 8) as u8;
    //         buf[i + 2] = (rand >> 16) as u8;
    //         buf[i + 3] = (rand >> 24) as u8;
    //         buf[i + 4] = (rand >> 32) as u8;
    //         buf[i + 5] = (rand >> 40) as u8;
    //         buf[i + 6] = (rand >> 48) as u8;
    //         buf[i + 7] = (rand >> 56) as u8;
    //         i += 8;
    //     }
    //
    //     out.write(&buf).unwrap();
    // }

    enum Generator {
        Xorshift,
        Wyrand,
        Cycle
    }

    impl FromStr for Generator {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "xorshift" => Ok(Self::Xorshift),
                "wyrand" => Ok(Self::Wyrand),
                "cycle" => Ok(Self::Cycle),
                _ => Err(format!("unknown generator '{}'", s))
            }
        }
    }

    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: {} <generator> <count>", args[0]);
        exit(1);
    }

    let generator = Generator::from_str(&args[1]).unwrap();
    let count = usize::from_str(&args[2]).unwrap();
    println!("#==================================================================");
    println!("# generator {}", "wyrand");
    println!("#==================================================================");
    println!("type: d");
    println!("count: {count}");
    println!("numbit: 64");
    for _ in 0..count {
        let rand = w.next_u64();
        println!("{}", rand);
    }
}