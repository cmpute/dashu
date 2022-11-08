extern crate clap;
use clap::{App, AppSettings, Arg, SubCommand};
use number::Number;
use std::time::{Duration, Instant};

mod digits_of_e;
mod fib;
mod number;

fn main() {
    let args = App::new("Bigint benchmarks")
        .arg(
            Arg::with_name("lib")
                .long("lib")
                .possible_values(&[
                    "ibig",
                    "dashu",
                    "num-bigint",
                    "ramp",
                    "rug",
                    "rust-gmp",
                    "malachite",
                ])
                .multiple(true)
                .number_of_values(1)
                .required(true)
                .min_values(1),
        )
        .arg(
            Arg::with_name("task")
                .long("task")
                .takes_value(true)
                .possible_values(&["e", "fib", "fib_hex"])
                .required(true),
        )
        .arg(
            Arg::with_name("n")
                .short("n")
                .takes_value(true)
                .required(true),
        )
        .subcommand(SubCommand::with_name("print"))
        .subcommand(SubCommand::with_name("benchmark"))
        .settings(&[AppSettings::SubcommandRequired])
        .get_matches();

    let libs: Vec<String> = args
        .values_of("lib")
        .unwrap()
        .map(|s| s.to_string())
        .collect();
    let task = args.value_of("task").unwrap();
    let n: u32 = args.value_of("n").unwrap().parse().expect("invalid n");

    match args.subcommand() {
        ("print", _) => command_print(&libs, task, n),
        ("benchmark", _) => command_benchmark(&libs, task, n),
        _ => unreachable!(),
    }
}

fn command_print(libs: &[String], task: &str, n: u32) {
    let mut answer: Option<String> = None;
    for lib_name in libs {
        let (a, _) = run_task(lib_name, task, n, 1);
        match &answer {
            None => {
                println!("answer = {}", a);
                println!("{:10} agrees", lib_name);
                answer = Some(a);
            }
            Some(ans) => {
                if *ans == a {
                    println!("{:10} agrees", lib_name);
                } else {
                    println!("{} disagrees!", lib_name);
                }
            }
        }
    }
}

fn command_benchmark(libs: &[String], task: &str, n: u32) {
    let mut answer: Option<String> = None;
    let mut results: Vec<(&String, Duration)> = Vec::new();
    for lib_name in libs {
        println!("{}", lib_name);
        // Take the median of 5 attempts, each attempt at least 10 seconds.
        let mut durations: Vec<Duration> = Vec::new();
        for sample_number in 0..5 {
            let mut iter = 0;
            let mut duration = Duration::from_secs(0);
            while duration < Duration::from_secs(10) {
                let i = iter.max(1);
                let (a, d) = run_task(lib_name, task, n, i);
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
        results.push((lib_name, duration));
    }
    results.sort_by_key(|&(_, d)| d);
    println!("Results");
    for (lib_name, duration) in results {
        println!("{:10} {} ms", lib_name, duration.as_millis());
    }
}

fn run_task(lib: &str, task: &str, n: u32, iter: u32) -> (String, Duration) {
    match lib {
        "ibig" => run_task_using::<ibig::UBig>(task, n, iter),
        "dashu" => run_task_using::<dashu_int::UBig>(task, n, iter),
        "num-bigint" => run_task_using::<num_bigint::BigUint>(task, n, iter),
        #[cfg(feature = "ramp")]
        "ramp" => run_task_using::<ramp::Int>(task, n, iter),
        "rug" => run_task_using::<rug::Integer>(task, n, iter),
        "rust-gmp" => run_task_using::<gmp::mpz::Mpz>(task, n, iter),
        "malachite" => run_task_using::<malachite_nz::natural::Natural>(task, n, iter),
        #[cfg(feature = "ramp")]
        _ => unreachable!(),
        #[cfg(not(feature = "ramp"))]
        _ => unreachable!("ramp is only supported with nightly rust!"),
    }
}

fn run_task_using<T: Number>(task: &str, n: u32, iter: u32) -> (String, Duration) {
    let mut answer = None;
    let start_time = Instant::now();
    for _ in 0..iter {
        let a = match task {
            "e" => digits_of_e::calculate::<T>(n),
            "fib" => fib::calculate_decimal::<T>(n),
            "fib_hex" => fib::calculate_hex::<T>(n),
            _ => unreachable!(),
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
// - fibonacci reciprocal a_n = a_n-1 + 1 / a_n-2
