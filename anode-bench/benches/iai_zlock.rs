use anode::remedy::Remedy;
use anode::zlock::{ReadBiased, Stochastic, WriteBiased, ZLock};
use iai::{black_box, main};
use std::sync::RwLock;

fn read_biased_read() {
    let lock = ZLock::<_, ReadBiased>::new(());
    black_box(lock.read());
}

fn read_biased_write() {
    let lock = ZLock::<_, ReadBiased>::new(());
    black_box(lock.write());
}

fn write_biased_read() {
    let lock = ZLock::<_, WriteBiased>::new(());
    black_box(lock.read());
}

fn write_biased_write() {
    let lock = ZLock::<_, WriteBiased>::new(());
    black_box(lock.write());
}

fn stochastic_read() {
    let lock = ZLock::<_, Stochastic>::new(());
    black_box(lock.read());
}

fn stochastic_write() {
    let lock = ZLock::<_, Stochastic>::new(());
    black_box(lock.write());
}

fn std_read() {
    let lock = RwLock::new(());
    let _ = black_box(lock.read().remedy());
}

fn std_write() {
    let lock = RwLock::new(());
    let _ = black_box(lock.write().remedy());
}

main!(
    read_biased_read,
    read_biased_write,
    write_biased_read,
    write_biased_write,
    stochastic_read,
    stochastic_write,
    std_read,
    std_write
);
