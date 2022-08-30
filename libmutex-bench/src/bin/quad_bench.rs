use std::sync::{Mutex, RwLock};
use libmutex::xlock::{ArrivalOrdered, LegacyReadBiased, LegacyWriteBiased, Stochastic};
use libmutex::xlock::ReadBiased;
use libmutex::xlock::WriteBiased;
use libmutex::xlock::XLock;
use libmutex_bench::quad_harness::{ExtendedOptions, Options};
use libmutex_bench::{args, quad_harness};
use std::time::Duration;
use libmutex::spinlock::SpinLock;
use libmutex_bench::lock_spec::LockSpec;
use libmutex_bench::quad_harness::print::{Header, Separator};

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
                        run::<XLock::<_, ReadBiased>>("synchrony::rwlock::RwLock<ReadBiased>", &opts);
                        run::<XLock::<_, LegacyReadBiased>>("synchrony::rwlock::RwLock<LegacyReadBiased>", &opts);
                        run::<XLock::<_, WriteBiased>>("synchrony::rwlock::RwLock<WriteBiased>", &opts);
                        run::<XLock::<_, LegacyWriteBiased>>("synchrony::rwlock::RwLock<LegacyWriteBiased>", &opts);
                        run::<XLock::<_, ArrivalOrdered>>("synchrony::rwlock::RwLock<ArrivalOrdered>", &opts);
                        run::<XLock::<_, Stochastic>>("synchrony::rwlock::RwLock<Stochastic>", &opts);
                        run::<SpinLock<_>>("synchrony::spin_mutex::SpinMutex", &opts);
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
