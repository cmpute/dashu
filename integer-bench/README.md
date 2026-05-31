Criterion benchmarks comparing [`dashu-int`](../integer) against other Rust
big-integer libraries. Each bench body is written once, generic over a
`Backend`, and run for every library, with the backend name as a `BenchmarkId`
dimension — so a single run reports all libraries side-by-side in one group.

This is a standalone crate, deliberately kept out of the dashu workspace (see
the root `Cargo.toml` `exclude`). That means it never gets pulled into
`dashu-int`'s `--all-features` builds or its MSRV check, and it is free to
require a newer toolchain than dashu (malachite needs Rust 1.90).

## Libraries

| Library                                          | Version | Notes                                          |
| -------                                          | ------- | -----                                          |
| [dashu-int](https://crates.io/crates/dashu-int)  | (path)  | The library under test. Pure Rust, no_std      |
| [ibig](https://crates.io/crates/ibig)            | 0.3     | Pure Rust, no_std. dashu-int's ancestor        |
| [num-bigint](https://crates.io/crates/num-bigint)| 0.4     | Pure Rust. The de-facto standard               |
| [malachite](https://crates.io/crates/malachite)  | 0.9     | Pure Rust, LGPL, derived from GMP and FLINT    |
| [rug](https://crates.io/crates/rug)              | 1.30    | Links [GMP](https://gmplib.org/); `gmp` feature |

The pure-Rust backends (dashu, ibig, num-bigint, malachite) are always built.
rug is added under the `gmp` feature, which needs the GMP toolchain.

## Benchmarks

| Bench       | What it covers                                                                 |
| -----       | -------------                                                                  |
| `primitive` | Bit-width sweep (10^1..10^4 bits): add/sub/mul/div, gcd, pow, radix, modular   |
| `small_int` | Small / inline-magnitude values (≤ 128 bits) the bit-width sweep under-covers  |
| `workload`  | Generator/shrinker-style scenarios: running sums, string round-trips, op mixes |
| `shrinker`  | The bigint operations a property-based-testing shrinker performs               |

The `workload` and `shrinker` shapes were drawn from profiling
[hegel-rust](https://github.com/hegeldev/hegel-rust) but are written to stand on
their own.

The `primitive` `ubig_modulo_*` benches measure each library's plain
multiply-then-reduce and native modpow (nothing precomputed), for a
like-for-like comparison.

## Usage

Run from the repository root with `--manifest-path`, or from inside this
directory:

```sh
# All pure-Rust backends (no toolchain needed):
cargo bench --manifest-path integer-bench/Cargo.toml

# A single bench, quickly:
cargo bench --manifest-path integer-bench/Cargo.toml --bench small_int -- --quick

# Include the rug (GMP) backend (needs the GMP toolchain):
cargo bench --manifest-path integer-bench/Cargo.toml --features gmp

# Smoke test only — build and run each bench once, no measurement:
cargo bench --manifest-path integer-bench/Cargo.toml -- --test
```

## License

Part of the [dashu](..) project; dual-licensed under
[MIT](../LICENSE-MIT) or [Apache-2.0](../LICENSE-APACHE).
