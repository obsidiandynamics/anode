use std::sync::{Mutex, RwLock};
use anode::zlock::{ArrivalOrdered, LegacyReadBiased, LegacyWriteBiased, Stochastic};
use anode::zlock::ReadBiased;
use anode::zlock::WriteBiased;
use anode::zlock::ZLock;
use anode_bench::quad_harness::{ExtendedOptions, Options};
use anode_bench::{args, quad_harness};
use std::time::Duration;
use anode::spinlock::SpinLock;
use anode_bench::lock_spec::LockSpec;
use anode_bench::quad_harness::print::{Header, Separator};

fn main() {
    let args = args::parse(&["readers", "writers", "downgraders", "upgraders", "duration"]);

    for readers in args[0] {
        for writers in args[1] {
            for downgraders in args[2] {
                for upgraders in args[3] {
                    for duration in args[4] {
                        let opts = Options {
                            readers,
                            writers,
                            downgraders,
                            upgraders,
                            duration: Duration::from_secs(duration as u64),
                        };
                        println!("{}", Separator());
                        println!("{}", opts);
                        println!("{}", Header());
                        run::<ZLock::<_, ReadBiased>>("anode::rwlock::RwLock<ReadBiased>", &opts);
                        run::<ZLock::<_, LegacyReadBiased>>("anode::rwlock::RwLock<LegacyReadBiased>", &opts);
                        run::<ZLock::<_, WriteBiased>>("anode::rwlock::RwLock<WriteBiased>", &opts);
                        run::<ZLock::<_, LegacyWriteBiased>>("anode::rwlock::RwLock<LegacyWriteBiased>", &opts);
                        run::<ZLock::<_, ArrivalOrdered>>("anode::rwlock::RwLock<ArrivalOrdered>", &opts);
                        run::<ZLock::<_, Stochastic>>("anode::rwlock::RwLock<Stochastic>", &opts);
                        run::<SpinLock<_>>("anode::spin_mutex::SpinMutex", &opts);
                        run::<RwLock<_>>("std::sync::RwLock", &opts);
                        run::<Mutex<_>>("std::sync::Mutex", &opts);
                    }
                }
            }
        }
    }
    println!("{}", Separator());
}

fn run<L: for <'a> LockSpec<'a, T=i64> + 'static>(name: &str, opts: &Options) {
    let ext_opts = ExtendedOptions {
        // stick your overrides here
        ..ExtendedOptions::default()
    };
    let result = quad_harness::run::<i64, L>(opts, &ext_opts);
    println!("|{:45}|{result}", name);
}
