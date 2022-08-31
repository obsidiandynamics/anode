use anode_bench::{pl_harness, pl_shims};
use anode_bench::pl_harness::RwLock;

#[test]
fn pl_micro_bench_read_biased() {
    __pl_micro_bench::<pl_shims::ReadBiasedLock<_>>();
}

#[test]
fn pl_micro_bench_write_biased() {
    __pl_micro_bench::<pl_shims::WriteBiasedLock<_>>();
}

#[test]
fn pl_micro_bench_arrival_ordered() {
    __pl_micro_bench::<pl_shims::ArrivalOrderedLock<_>>();
}

#[test]
fn pl_micro_bench_stochastic() {
    __pl_micro_bench::<pl_shims::StochasticLock<_>>();
}

fn __pl_micro_bench<M: RwLock<f64> + Send + Sync + 'static>() {
    pl_harness::run_benchmark_iterations::<M>(2, 2, 1, 1, 0.1, 1);
}