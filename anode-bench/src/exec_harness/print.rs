use std::fmt::{Display, Formatter};
use crate::exec_harness::{BenchmarkResult, Options};
use crate::rate::Elapsed;

impl Display for Options {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "|{:>45}|{:>20}|",
            "duration",
            format!("{:?}", self.duration),
        )
    }
}

impl Display for BenchmarkResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let work_rate = self.rate(self.iterations);
        write!(
            f,
            "{:>20}|",
            format!("{:.3}", work_rate.khz()),
        )
    }
}

pub struct Separator();

impl Display for Separator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "|{:->45}|{:->20}|",
            "", "",
        )
    }
}

pub struct Header();

impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "|{:45}|{:>20}|",
            "", "rate (kHz)"
        )
    }
}