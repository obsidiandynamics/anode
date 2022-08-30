use std::fmt::Formatter;
use std::fmt::Display;

#[derive(Debug)]
pub struct Rate(pub f64);

impl Rate {
    pub fn hz(&self) -> f64 {
        self.0
    }

    pub fn khz(&self) -> f64 {
        self.0 / 1_000.0
    }

    pub fn mhz(&self) -> f64 {
        self.0 / 1_000_000.0
    }
}

impl Display for Rate {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut unaligned = {
            if f.alternate() {
                format!("{:.3} kHz", self.khz())
            } else {
                match self.0 {
                    val if val > 1_000_000.0 => format!("{:.3} MHz", self.mhz()),
                    val if val > 1_000.0 => format!("{:.3} kHz", self.khz()),
                    _ => format!("{:.3} Hz", self.hz()),
                }
            }
        };

        if let Some(width) = f.width() {
            while unaligned.len() < width {
                unaligned.insert(0, ' ');
            }
        }
        f.write_str(&unaligned)
    }
}