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

## Usage examples

- Print results:
    - Integer: `cargo run -- --lib dashu --lib num --lib malachite --lib ibig --task e -n 100 print`
    - Rational: `cargo run -- --lib dashu --lib num --lib malachite --task fib_ratio -n 100 print`
    - Decimal Float: `cargo run -- --lib dashu --lib bigdecimal --task e_decimal -n 100 print`
    - Binary Float (π): `cargo run -- --lib dashu --lib astro_float --task pi -n 100 print`
    - Binary Float (π, incl. GMP): `cargo run --features gmp -- --lib dashu --lib astro_float --lib rug --task pi -n 100 print`
- Run the benchmark: change `print` to `exec` in the commands above and select a larger `n`.
