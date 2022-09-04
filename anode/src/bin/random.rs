use std::{env, io};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::{ErrorKind, stdout, Write};
use std::process::exit;
use std::str::FromStr;
use anode::rand::{Rand64, Wyrand, Xorshift};

fn main() {
    if let Err(err) = generate() {
        eprintln!("Error: {}", err);
        exit(1);
    }
}

#[derive(Debug)]
struct GeneratorError(String);

impl Display for GeneratorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for GeneratorError {}

enum Generator {
    Xorshift,
    Wyrand,
    Cycle
}

impl FromStr for Generator {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "xorshift" => Ok(Self::Xorshift),
            "wyrand" => Ok(Self::Wyrand),
            "cycle" => Ok(Self::Cycle),
            _ => Err(format!("unknown generator '{}'", s))
        }
    }
}

enum OutputFormat {
    Text,
    Binary
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(Self::Text),
            "binary" => Ok(Self::Binary),
            _ => Err(format!("unknown output format '{}'", s))
        }
    }
}

#[derive(Default)]
struct Cycle(u64);

impl Rand64 for Cycle {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(1);
        self.0
    }
}

fn generate() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: {} <generator ∈ {{xorshift, wyrand}}> <format ∈ {{text, binary}}> <count ∈ ℕ⁺>", args[0]);
        exit(1);
    }

    let generator = Generator::from_str(&args[1])?;
    let format = OutputFormat::from_str(&args[2])?;
    let count = &args[3];
    let count = count.replace("K", "000");
    let count = count.replace("M", "000000");
    let count = count.replace("B", "000000000");
    let count = count.replace("G", "000000000");
    let count = count.replace("T", "000000000000");
    let count = u64::from_str(&count)?;

    let rand: Box<dyn Rand64> = match generator {
        Generator::Xorshift => Box::new(Xorshift::default()),
        Generator::Wyrand => Box::new(Wyrand::default()),
        Generator::Cycle => Box::new(Cycle::default()),
    };

    match format {
        OutputFormat::Text => generate_text(&args[1], count, rand),
        OutputFormat::Binary => generate_bin(count, rand)
    }
}

fn generate_text(rand_name: &str, count: u64, mut rand: Box<dyn Rand64>) -> Result<(), Box<dyn Error>> {
    println!("#==================================================================");
    println!("# generator {}", rand_name);
    println!("#==================================================================");
    println!("type: d");
    println!("count: {count}");
    println!("numbit: 64");
    for _ in 0..count {
        let random = rand.next_u64();
        println!("{}", random);
    }
    Ok(())
}

fn generate_bin(count: u64, mut rand: Box<dyn Rand64>) -> Result<(), Box<dyn Error>> {
    let mut out = stdout();
    let mut buf = [0u8; 8];
    let mut samples = 0;
    loop {
        let rand = rand.next_u64();
        buf[0] = rand as u8;
        buf[1] = (rand >> 8) as u8;
        buf[2] = (rand >> 16) as u8;
        buf[3] = (rand >> 24) as u8;
        buf[4] = (rand >> 32) as u8;
        buf[5] = (rand >> 40) as u8;
        buf[6] = (rand >> 48) as u8;
        buf[7] = (rand >> 56) as u8;
        out.write(&buf).suppress()?;

        samples += 1;
        if samples == count {
            break;
        }
    }
    Ok(())
}

trait SuppressBrokenPipe {
    fn suppress(self) -> io::Result<usize>;
}

impl SuppressBrokenPipe for io::Result<usize> {
    fn suppress(self) -> io::Result<usize> {
        match self {
            Ok(_) => self,
            Err(err) => {
                match err.kind() {
                    ErrorKind::BrokenPipe => Ok(0),
                    _ => Err(err),
                }
            }
        }
    }
}