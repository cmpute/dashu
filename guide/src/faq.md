# FAQ

## Why is the library called `dashu`?

`dashu` is the pinyin romanization of 大数 ("dà shù"), Chinese for *big number*.

## Why to use `dashu`?

`dashu` aims to be a Rust-native, ergonomic alternative to GNU GMP + MPFR + MPC: arbitrary-precision integers, floats, rationals, and complex numbers, all in pure Rust with full `no_std` support and arbitrary-base floats.

Compared with other Rust crates:

| Crate | Pure Rust | Full `no_std` | Int | Float | Ratio | Complex |
|-------|-----------|---------------|-----|-------|-------|---------|
| **dashu** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `num-bigint` | ✓ | ✗ | ✓ | ✗ | ✗ | ✗ |
| `ibig` | ✓ | ✓ | ✓ | ✗ | ✗ | ✗ |
| `rug` | ✗ (C/GMP) | ✗ | ✓ | ✓ | ✓ | ✓ |

`malachite` also offers pure-Rust integers and rationals with a performance focus, but is `std`-oriented and does not cover arbitrary-precision floats or complex numbers. Unlike `rug`, `dashu` has no C dependency — it builds and runs anywhere Rust does, including `no_std` targets.

## Known limitations

- **No NaN.** Invalid operations panic at the convenience layer and return `Err(FpError)` at the context layer. Infinities are terminal values, not operands — see [Standards Compliance](./compliance.md).
- **Near-correct rounding.** Transcendentals are rounded within 1 ulp via a guard-digit recipe; a guaranteed-correct Ziv loop is planned for a later release.
- **Complex surface.** `CBig` ships field arithmetic and the elementary transcendentals; complex hyperbolics, `fma`, and several others are deferred to 0.5.x (see the v0.5 release notes).
- **No SIMD-FFT multiplication** yet (planned for v1.0).

## MSRV and feature policy

The current MSRV is **1.68**. Third-party integrations follow a versioned-feature convention: stable dependencies use `xxx_vYY` (e.g. `rand_v08`) with an unversioned `xxx` alias pinned to one version, while unstable dependencies alias `xxx` to the newest. See [Cargo Features](./index.md#cargo-features) for the full explanation.
