use std::time::{Duration, Instant};

use clap::ValueEnum as _;
use number::{Float, Natural, Rational};

mod e;
mod fib;
mod number;

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
    FibRatio,
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
                if *ans == a {
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
                    Some(ans) => assert!(*ans == a),
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
            Task::E | Task::Fib | Task::FibHex => {
                run_int_task_using::<dashu::Natural>(task, n, iter)
            }
            Task::FibRatio => run_ratio_task_using::<dashu::Rational>(task, n, iter),
            Task::DecimalE => run_decimal_task_using::<dashu::Decimal>(task, n, iter),
        },
        Lib::Num => match task {
            Task::E | Task::Fib | Task::FibHex => run_int_task_using::<num::BigUint>(task, n, iter),
            Task::FibRatio => run_ratio_task_using::<num::BigRational>(task, n, iter),
            Task::DecimalE => {
                panic!("Num crates don't support arbitrary precision float numbers yet.")
            }
        },
        #[cfg(feature = "ramp")]
        Lib::Ramp => run_int_task_using::<ramp::Int>(task, n, iter),
        #[cfg(feature = "rug")]
        Lib::Rug => run_int_task_using::<rug::Integer>(task, n, iter),
        #[cfg(feature = "rust-gmp")]
        Lib::RustGmp => run_int_task_using::<gmp::mpz::Mpz>(task, n, iter),
        Lib::Malachite => match task {
            Task::E | Task::Fib | Task::FibHex => {
                run_int_task_using::<malachite::Natural>(task, n, iter)
            }
            Task::FibRatio => run_ratio_task_using::<malachite::Rational>(task, n, iter),
            Task::DecimalE => {
                panic!("Malachite crates don't support arbitrary precision float numbers yet.")
            }
        },
        Lib::BigDecimal => run_decimal_task_using::<bigdecimal::BigDecimal>(task, n, iter),
    }
}

fn run_int_task_using<T: Natural>(task: Task, n: u32, iter: u32) -> (String, Duration) {
    let mut answer = None;
    let start_time = Instant::now();
    for _ in 0..iter {
        let a = match task {
            Task::E => e::calculate::<T>(n),
            Task::Fib => fib::calculate_decimal::<T>(n),
            Task::FibHex => fib::calculate_hex::<T>(n),
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

fn run_ratio_task_using<T: Rational>(task: Task, n: u32, iter: u32) -> (String, Duration) {
    let mut answer: Option<String> = None;
    let start_time = Instant::now();
    for _ in 0..iter {
        let a = match task {
            Task::FibRatio => fib::calculate_ratio::<T>(n),
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

fn run_decimal_task_using<T: Float>(task: Task, n: u32, iter: u32) -> (String, Duration) {
    let mut answer: Option<String> = None;
    let start_time = Instant::now();
    for _ in 0..iter {
        let a = match task {
            Task::DecimalE => T::e(n).to_string(),
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

// TODO: add task to test more operations, such as
// - some complex calculation: a=2^n, b=3^n, sqrt((a+b)/(a-b)).gcd(sqrt((a+b)*(a-b))
// - io: parse input and do square, then output
