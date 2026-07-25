use std::{
    fmt::Debug,
    str::FromStr,
    time::{Duration, Instant},
};

use clap::ValueEnum as _;
use number::{AstroFloat, Float, Natural, Rational};

mod e;
mod fib;
mod io;
mod number;
mod pi;

#[derive(clap::Parser)]
#[command(name = "Bigint benchmarks")]
struct Cli {
    #[arg(long = "lib", required = true)]
    libs: Vec<Lib>,
    #[arg(long = "task")]
    task: Task,
    #[arg(short = 'n')]
    n: u32,

    #[command(subcommand)]
    subcommand: SubCommand,
}

#[derive(Copy, Clone, clap::ValueEnum)]
enum Lib {
    #[value(name = "ibig")]
    IBig,
    #[value(name = "dashu")]
    Dashu,
    #[value(name = "num")]
    Num,
    #[cfg(feature = "ramp")]
    #[value(name = "ramp")]
    Ramp,
    #[cfg(feature = "rug")]
    #[value(name = "rug")]
    Rug,
    #[cfg(feature = "rust-gmp")]
    #[value(name = "rust-gmp")]
    RustGmp,
    #[value(name = "malachite")]
    Malachite,
    #[value(name = "bigdecimal")]
    BigDecimal,
    #[value(name = "astro_float")]
    AstroFloat,
}

#[derive(Copy, Clone, clap::ValueEnum)]
enum Task {
    #[value(name = "e")]
    E,
    #[value(name = "e_decimal")]
    DecimalE,
    #[value(name = "fib")]
    Fib,
    #[value(name = "fib_hex")]
    FibHex,
    #[value(name = "fib_ratio")]
    FibRational,
    #[value(name = "io_int")]
    IntegerIO,
    #[value(name = "io_ratio")]
    RationalIO,
    #[value(name = "io_decimal")]
    DecimalIO,
    #[value(name = "pi")]
    Pi,
}

#[derive(clap::Subcommand)]
enum SubCommand {
    #[command(name = "print")]
    Print,
    #[command(name = "exec")]
    Execute,
}

fn main() {
    let args: Cli = clap::Parser::parse();

    match args.subcommand {
        SubCommand::Print => command_print(&args.libs, args.task, args.n),
        SubCommand::Execute => command_benchmark(&args.libs, args.task, args.n),
    }
}

fn command_print(libs: &[Lib], task: Task, n: u32) {
    let mut answer: Option<String> = None;
    for &lib in libs {
        let lib_name = lib.to_possible_value().unwrap();
        let (a, _) = run_task(lib, task, n, 1);
        match &answer {
            None => {
                println!("answer = {}", a);
                println!("{:10} agrees", lib_name.get_name());
                answer = Some(a);
            }
            Some(ans) => {
                if results_match(task, ans, &a) {
                    println!("{:10} agrees", lib_name.get_name());
                } else {
                    println!("{} disagrees!", lib_name.get_name());
                }
            }
        }
    }
}

fn command_benchmark(libs: &[Lib], task: Task, n: u32) {
    let mut answer: Option<String> = None;
    let mut results: Vec<(Lib, Duration)> = Vec::new();
    for &lib in libs {
        let lib_name = lib.to_possible_value().unwrap();
        println!("{}", lib_name.get_name());
        // Take the median of 5 attempts, each attempt at least 10 seconds.
        let mut durations: Vec<Duration> = Vec::new();
        for sample_number in 0..5 {
            let mut iter = 0;
            let mut duration = Duration::from_secs(0);
            while duration < Duration::from_secs(10) {
                let i = iter.max(1);
                let (a, d) = run_task(lib, task, n, i);
                match &answer {
                    None => answer = Some(a),
                    Some(ans) => assert!(results_match(task, ans, &a)),
                }
                iter += i;
                duration += d;
            }
            let duration = duration / iter;
            println!("Attempt {}: {} iterations {} ms", sample_number, iter, duration.as_millis());
            durations.push(duration);
        }
        durations.sort();
        let duration = durations[0];
        results.push((lib, duration));
    }
    results.sort_by_key(|&(_, d)| d);
    println!("Results");
    for (lib, duration) in results {
        let lib_name = lib.to_possible_value().unwrap();
        println!("{:10} {} ms", lib_name.get_name(), duration.as_millis());
    }
}

fn run_task(lib: Lib, task: Task, n: u32, iter: u32) -> (String, Duration) {
    match lib {
        Lib::IBig => run_int_task_using::<ibig::UBig>(task, n, iter),
        Lib::Dashu => match task {
            Task::E | Task::Fib | Task::FibHex | Task::IntegerIO => {
                run_int_task_using::<dashu::Natural>(task, n, iter)
            }
            Task::FibRational | Task::RationalIO => {
                run_ratio_task_using::<dashu::Rational>(task, n, iter)
            }
            Task::DecimalE | Task::DecimalIO => {
                run_decimal_task_using::<dashu::Decimal>(task, n, iter)
            }
            Task::Pi => run_float_task_using::<dashu::Real>(n, iter),
        },
        Lib::Num => match task {
            Task::E | Task::Fib | Task::FibHex | Task::IntegerIO => {
                run_int_task_using::<num::BigUint>(task, n, iter)
            }
            Task::FibRational | Task::RationalIO => {
                run_ratio_task_using::<num::BigRational>(task, n, iter)
            }
            Task::DecimalE | Task::DecimalIO | Task::Pi => {
                panic!("Num crates don't support arbitrary precision float numbers yet.")
            }
        },
        #[cfg(feature = "ramp")]
        Lib::Ramp => run_int_task_using::<ramp::Int>(task, n, iter),
        #[cfg(feature = "rug")]
        Lib::Rug => match task {
            Task::Pi => run_float_task_using::<rug::Float>(n, iter),
            _ => run_int_task_using::<rug::Integer>(task, n, iter),
        },
        #[cfg(feature = "rust-gmp")]
        Lib::RustGmp => run_int_task_using::<gmp::mpz::Mpz>(task, n, iter),
        Lib::Malachite => match task {
            Task::E | Task::Fib | Task::FibHex | Task::IntegerIO => {
                run_int_task_using::<malachite::Natural>(task, n, iter)
            }
            Task::FibRational | Task::RationalIO => {
                run_ratio_task_using::<malachite::Rational>(task, n, iter)
            }
            Task::DecimalE | Task::DecimalIO | Task::Pi => {
                panic!("Malachite crates don't support arbitrary precision float numbers yet.")
            }
        },
        Lib::BigDecimal => run_decimal_task_using::<bigdecimal::BigDecimal>(task, n, iter),
        Lib::AstroFloat => match task {
            Task::Pi => run_float_task_using::<AstroFloat>(n, iter),
            _ => panic!("astro_float only participates in the pi task"),
        },
    }
}

fn run_int_task_using<T: Natural>(task: Task, n: u32, iter: u32) -> (String, Duration)
where
    <T as FromStr>::Err: Debug,
{
    let mut answer = None;
    let start_time = Instant::now();
    for _ in 0..iter {
        let a = match task {
            Task::E => e::calculate::<T>(n),
            Task::Fib => fib::calculate_decimal::<T>(n),
            Task::FibHex => fib::calculate_hex::<T>(n),
            Task::IntegerIO => io::calculate_natural::<T>(n),
            _ => panic!("One of the libraries is not adapted to integer benchmarks!"),
        };
        match &answer {
            None => answer = Some(a),
            Some(ans) => assert!(a == *ans),
        }
    }
    let time = start_time.elapsed();
    (answer.unwrap(), time)
}

fn run_ratio_task_using<T: Rational>(task: Task, n: u32, iter: u32) -> (String, Duration)
where
    <T as FromStr>::Err: Debug,
{
    let mut answer: Option<String> = None;
    let start_time = Instant::now();
    for _ in 0..iter {
        let a = match task {
            Task::FibRational => fib::calculate_rational::<T>(n),
            Task::RationalIO => io::calculate_ratioal::<T>(n),
            _ => panic!("One of the libraries is not adapted to rational benchmarking!"),
        };
        match &answer {
            None => answer = Some(a),
            Some(ans) => assert!(a == *ans),
        }
    }
    let time = start_time.elapsed();
    (answer.unwrap(), time)
}

fn run_decimal_task_using<T: Float + FromStr + From<u32>>(
    task: Task,
    n: u32,
    iter: u32,
) -> (String, Duration)
where
    <T as FromStr>::Err: Debug,
{
    let mut answer: Option<String> = None;
    let start_time = Instant::now();
    for _ in 0..iter {
        let a = match task {
            Task::DecimalE => T::e(n).to_string(),
            Task::DecimalIO => io::calculate_decimal::<T>(n),
            _ => panic!("One of the libraries is not adapted to float benchmarking!"),
        };
        match &answer {
            None => answer = Some(a),
            Some(ans) => assert!(a == *ans),
        }
    }
    let time = start_time.elapsed();
    (answer.unwrap(), time)
}

/// Binary-float benchmark runner (the `pi` task). Only needs `Float` (no
/// `FromStr`/`From<u32>`), so libraries like `rug::Float` — which don't impl
/// those — can still participate.
fn run_float_task_using<T: Float>(n: u32, iter: u32) -> (String, Duration) {
    // Word-align the precision so dashu (exact-bit) and astro-float (which
    // rounds precision up to the word size) carry the same number of sig bits.
    let bits = (n as usize).div_ceil(64) * 64;
    let mut answer: Option<String> = None;
    let start_time = Instant::now();
    for _ in 0..iter {
        let a = pi::calculate::<T>(bits as u32);
        match &answer {
            None => answer = Some(a),
            Some(ans) => assert!(a == *ans),
        }
    }
    let time = start_time.elapsed();
    (answer.unwrap(), time)
}

/// Cross-library agreement check. All tasks use exact string equality except
/// `pi`, whose [`pi::within_tolerance`] allows a few ULP of slack (different
/// libraries round differently).
fn results_match(task: Task, a: &str, b: &str) -> bool {
    match task {
        Task::Pi => pi::within_tolerance(a, b),
        _ => a == b,
    }
}
