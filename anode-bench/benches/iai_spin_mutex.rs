use iai::{black_box, main};
use anode::spin_mutex::SpinMutex;

fn lock() {
    let lock = SpinMutex::new(());
    black_box(lock.lock());
}

main!(lock);