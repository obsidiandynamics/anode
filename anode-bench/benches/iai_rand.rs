use iai::{main};
use anode::rand::{Probability, Rand, Xorshift};

fn xorshift_next_u64() -> u64 {
    let mut rand = Xorshift::default();
    rand.next_u64()
}

fn xorshift_gen_bool() -> bool {
    let mut rand = Xorshift::default();
    rand.next_bool(Probability::new(0.5))
}

main!(xorshift_next_u64, xorshift_gen_bool);