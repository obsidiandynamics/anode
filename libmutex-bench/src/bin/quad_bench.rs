use libmutex::xlock::{ArrivalOrdered, Moderator, ReadBiased, WriteBiased, XLock};
use libmutex_bench::quad_harness::{ExtendedOptions, Options};
use libmutex_bench::{args, quad_harness};
use std::time::Duration;

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
                        println!(">>> {opts:?}");

                        run::<ReadBiased>("RwLock<ReadBiased>", opts.clone());
                        run::<WriteBiased>("RwLock<WriteBiased>", opts.clone());
                        run::<ArrivalOrdered>("RwLock<ArrivalOrdered>", opts.clone());
                    }
                }
            }
        }
    }
}

fn run<M: Moderator + 'static>(
    name: &str,
    opts: Options,
) {
    let ext_opts = ExtendedOptions {
        ..ExtendedOptions::default()
    };
    let lock = XLock::<_, M>::new(0);
    let result = quad_harness::run(lock, opts, ext_opts);
    println!("|{:25}| {result}", name);
}
