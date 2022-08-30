use libmutex::rand::{clock_seed, Probability, Rand64, Seeded, Xorshift};

fn main() {
    //TODO
    let tries = 10000;
    let p = Probability::new(0.05);
    let mut rng = Xorshift::seed(clock_seed());
    let mut was_true = 0;
    for _ in 0..tries {
        let b = rng.gen_bool(p);
        if b {
            was_true += 1;
        }
    }
    let rate = was_true as f64 / tries as f64;
    println!("rate of true {}", rate);
}