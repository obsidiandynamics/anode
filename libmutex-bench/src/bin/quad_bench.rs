use libmutex::xlock::{ArrivalOrdered, Moderator, ReadBiased, WriteBiased, XLock};
use libmutex_bench::quad_harness::{Addable, ExtendedOptions, Options};
use libmutex_bench::{args, quad_harness};
use std::time::Duration;
use libmutex_bench::lock_spec::LockSpec;

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
                        println!("{opts:?}");

                        run("RwLock<ReadBiased>", XLock::<_, ReadBiased>::new(0), opts.clone());
                        run("RwLock<WriteBiased>", XLock::<_, WriteBiased>::new(0), opts.clone());
                        run("RwLock<ArrivalOrdered>", XLock::<_, ArrivalOrdered>::new(0), opts.clone());
                    }
                }
            }
        }
    }
}

fn run<T: Addable, L: for <'a> LockSpec<'a, T=T> + 'static>(name: &str, lock: L, opts: Options) {
    let ext_opts = ExtendedOptions {
        ..ExtendedOptions::default()
    };
    let result = quad_harness::run(lock, opts, ext_opts);
    println!("|{:25}| {result}", name);
}
