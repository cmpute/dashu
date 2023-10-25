Benchmarks for Rust big integer implementations. The code is adopted from [bigint-benchmark-rs](https://github.com/tczajka/bigint-benchmark-rs), and see [LICENSE](./LICENSE) for the original license.

## Libraries

| Library                                               | Version | Notes                                                  |
| --------------                                        | ------- | ------                                                 |
| [dashu](https://crates.io/crates/dashu)               | 0.4.0   | Pure Rust, no_std                                      |
| [rug](https://crates.io/crates/rug)                   | 1.22.0  | Links to libc and [GMP](https://gmplib.org/)           |
| [rust-gmp](https://crates.io/crates/rust-gmp)         | 0.5.0   | Links to libc and [GMP](https://gmplib.org/)           |
| [ibig](https://crates.io/crates/ibig)                 | 0.3.6   | Pure Rust, no_std                                      |
| [malachite-nz](https://crates.io/crates/malachite-nz) | 0.4.2   | Pure Rust, LGPL, derived from GMP and FLINT            |
| [num-bigint](https://crates.io/crates/num-bigint)     | 0.4.4   | Pure Rust, no_std                                      |
| [ramp](https://crates.io/crates/ramp)                 | 0.7.0   | Requires nightly Rust, uses x86_64 assembly            |

## Tasks

| Task      | Description                   | Difficulty | Algorithm             | Operations |
| ----      | ---------                     | ---------- | ---------             | ---------- |
| `e`       | n digits of e                 | Hard       | Binary splitting      | addition, multiplication, division, exponentiation, base conversion |
| `fib`     | n-th Fibonnaci number         | Medium     | Matrix exponentiation | addition, multiplication, base conversion |
| `fib_hex` | n-th Fibonnaci number in hex  | Easy       | Matrix exponentiation | addition, multiplication |

## Usage examples

- Print results: `cargo run -- --lib ibig --lib dashu --lib num-bigint --lib malachite --task e -n 100 print`
- Run the benchmark: `cargo run -- --lib ibig --lib dashu --lib num-bigint --lib malachite --task e -n 1000000 exec`
