//! Printing of options and results for the benchmark.

use std::fmt::{Display, Formatter};
use crate::quad_harness::{BenchmarkResult, Options};

impl Display for Options {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "|{:>45}|{:>20}|{:20}|{:>20}|{:>20}|\n|{:>45}|{:>20}|{:20}|{:>20}|{:>20}|\n|{:>45}|{:>20}|{:20}|{:>20}|{:>20}|",
            "readers",
            self.readers,
            "",
            "writers",
            self.writers,
            "downgraders",
            self.downgraders,
            "",
            "upgraders",
            self.upgraders,
            "duration",
            format!("{:?}", self.duration),
            "",
            "",
            ""
        )
    }
}

impl Display for BenchmarkResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let read_rate = self.rate(self.reads);
        let write_rate = self.rate(self.writes);
        let downgrade_rate = self.rate(self.downgrades);
        let upgrade_rate = self.rate(self.upgrades);
        write!(
            f,
            "{:>20.3}|{:>20.3}|{:>20.3}|{:>20.3}|",
            read_rate.khz(),
            write_rate.khz(),
            downgrade_rate.khz(),
            upgrade_rate.khz()
        )
    }
}

pub struct Separator();

impl Display for Separator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "|{:->45}|{:->20}|{:->20}|{:->20}|{:->20}|",
            "", "", "", "", ""
        )
    }
}

pub struct Header();

impl Display for Header {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "|{:45}|{:>20}|{:>20}|{:>20}|{:>20}|",
            "", "reads (kHz)", "writes (kHz)", "downgrades (kHz)", "upgrades (kHz)"
        )
    }
}