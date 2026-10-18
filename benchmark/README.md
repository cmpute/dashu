Benchmarks for arbitrary precision numbers with Rust implementations. The code is adopted from [bigint-benchmark-rs](https://github.com/tczajka/bigint-benchmark-rs), and see [LICENSE](./LICENSE) for the original license.

## Libraries

| Library                                               | Version | Notes                                                  | Supported Types                   |
| --------------                                        | ------- | ------                                                 | ---------------                   |
| [dashu](https://crates.io/crates/dashu)               | 0.4.0   | Pure Rust, no_std                                      | integer, float, decimal, rational |
| [rug](https://crates.io/crates/rug)                   | 1.30.0  | Links to libc and [GMP](https://gmplib.org/)           | integer, float, rational, complex |
| [rust-gmp](https://crates.io/crates/rust-gmp)         | 0.5.0   | Links to libc and [GMP](https://gmplib.org/)           | integer, float, rational          |
| [ibig](https://crates.io/crates/ibig)                 | 0.3.6   | Pure Rust, no_std                                      | integer                           |
| [malachite](https://crates.io/crates/malachite)       | 0.9.2   | Pure Rust, LGPL, derived from GMP and FLINT            | integer, rational                 |
| [num](https://crates.io/crates/num)                   | 0.4.3   | Pure Rust, no_std                                      | integer, rational, complex        |
| [ramp](https://crates.io/crates/ramp)                 | 0.7.0   | Requires nightly Rust, uses x86_64 assembly            | integer                           |
| [bigdecimal](https://crates.io/crates/bigdecimal)     | 0.4.10  | Pure Rust                                              | decimal                           |
| [astro-float](https://crates.io/crates/astro-float)    | 0.9.5   | Pure Rust, binary float                                | float (binary)                    |

## Tasks

| Task      | Description                   | Number Type | Difficulty | Algorithm             | Operations |
| ----      | ---------                     | ----------- | ---------- | ---------             | ---------- |
| `e`       | n digits of e                 | Integer     | Hard       | Binary splitting      | addition, multiplication, division, exponentiation, base conversion |
| `e_decimal` | n digits of e               | Decimal     | -          | Depends               | -          |
| `pi`        | n bits of π                 | Float (binary) | Medium | Arctangent identity | sqrt, division, atan, multiplication |
| `fib`     | n-th Fibonnaci number         | Integer     | Medium     | Matrix exponentiation | addition, multiplication, base conversion |
| `fib_hex` | n-th Fibonnaci number in hex  | Integer     | Easy       | Matrix exponentiation | addition, multiplication |
| `fib_ratio` | n-th modified Fibonnaci number | Rational | -          | -                     | -          |
| `factor`   | factor a semiprime with an n-bit and a ~31-bit prime factor | Integer | Medium | Pollard's rho (Brent) + Miller-Rabin | modular multiplication/exponentiation, gcd, division |

## Recommended parameters

Three sizes per task, chosen so a single iteration costs roughly the same across tasks
(~0.05 s / ~0.2 s / ~2.5-3.5 s on the reference machine, matching `factor` at 200/1000/5000).
Times below are dashu, one iteration:

| Task | Sizes (`n`) | dashu per iteration |
| ---- | ----------- | ------------------- |
| `e` | 100k / 500k / 4M | 0.04 / 0.21 / 3.7 s |
| `fib` | 1M / 4M / 30M | 0.03 / 0.20 / 3.3 s |
| `fib_hex` | 1M / 10M / 100M | 0.01 / 0.12 / 3.4 s |
| `io_int` | 100k / 1M / 5M | 0.01 / 0.28 / 2.6 s |
| `factor` | 200 / 1000 / 5000 | 0.04 / 0.20 / 2.7 s |
| `fib_ratio` | 2k / 6k / 20k | 0.02 / 0.16 / 3.4 s |
| `io_ratio` | 1M / 4M / 20M | 0.07 / 0.42 / 3.3 s |
| `io_decimal` | 100k / 1M / 5M | 0.01 / 0.27 / 2.4 s |
| `pi` | 10k / 20k / 50k (bits) | 0.07 / 0.35 / 3.1 s |

`e_decimal` has no size ladder: bigdecimal's precision is environment-controlled, so its
workload does not scale with `n`.

## Usage examples

- Print results:
    - Integer: `cargo run -- --lib dashu --lib num --lib malachite --lib ibig --task e -n 100 print`
    - Integer factorization (n = bits of the large prime factor; recommended 200 / 1000 / 5000): `cargo run --release -- --lib dashu --lib num --lib malachite --lib ibig --task factor -n 1000 print`
    - Rational: `cargo run -- --lib dashu --lib num --lib malachite --task fib_ratio -n 100 print`
    - Decimal Float: `cargo run -- --lib dashu --lib bigdecimal --task e_decimal -n 100 print`
    - Binary Float (π): `cargo run -- --lib dashu --lib astro_float --task pi -n 100 print`
    - Binary Float (π, incl. GMP): `cargo run --features gmp -- --lib dashu --lib astro_float --lib rug --task pi -n 100 print`
- Run the benchmark: change `print` to `exec` in the commands above and select a larger `n`.
- Compare all libraries on the recommended parameter ladder and plot the results: `cargo build --release --features gmp && python3 run.py`, then open `chart.html` (rug/rust-gmp need system GMP). Timings accumulate in `results.json` and are re-plottable with `python3 run.py --results results.json`; individual tasks can be (re)measured with `--tasks e,factor,...`.
