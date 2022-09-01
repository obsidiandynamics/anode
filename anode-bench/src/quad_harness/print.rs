//! Printing of options and results for the benchmark.

use std::fmt::{Display, Formatter};
use crate::quad_harness::{BenchmarkResult, Options};
use crate::rate::Rate;

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
        let read_rate = Rate::maybe_rate(self.elapsed, self.reads);
        let write_rate = Rate::maybe_rate(self.elapsed, self.writes);
        let downgrade_rate = Rate::maybe_rate(self.elapsed, self.downgrades);
        let upgrade_rate = Rate::maybe_rate(self.elapsed, self.upgrades);
        write!(
            f,
            "{:>20}|{:>20}|{:>20}|{:>20}|",
            read_rate.map_or(String::from("-"), |rate|format!("{:.3}", rate.khz())),
            write_rate.map_or(String::from("-"), |rate|format!("{:.3}", rate.khz())),
            downgrade_rate.map_or(String::from("-"), |rate|format!("{:.3}", rate.khz())),
            upgrade_rate.map_or(String::from("-"), |rate|format!("{:.3}", rate.khz())),
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