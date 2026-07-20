## Why is the library called `dashu`?

`dashu` is the pinyin romanization of 大数 ("dà shù"), Chinese for *big number*.

<p align="center">
  <img src="./assets/dashu-logo.png" alt="The dashu logo: a stylized banyan tree" width="200">
</p>

The logo is a stylized **banyan tree** — an archetypal *big tree* (大树, also "dà shù"). It's a
bilingual pun: 大树 (*big tree*) and 大数 (*big number*, the library's namesake) are homophones in
Mandarin, so a grand tree stands in visually for the "big number" behind the name. The downward
strokes are the banyan's signature [aerial roots](https://en.wikipedia.org/wiki/Aerial_root).

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
- **Correctly-rounded transcendentals.** The real transcendentals — `exp`, `exp_m1`, `ln`, `ln_1p`, the trigonometric and hyperbolic families, `hypot`, and `powf` (non-integer exponent) — are guaranteed-correctly rounded via a Ziv retry loop. An integer-valued `powf` exponent delegates to `powi` (binary exponentiation, within 1 ulp), and `dashu-cmplx`'s complex transcendental *wrappers* are still near-correct (within 1 ulp), routing through the now-correct real primitives.
- **Complex surface.** `CBig` ships field arithmetic and the elementary transcendentals; complex hyperbolics, `fma`, and several others are deferred to 0.5.x (see the v0.5 release notes).
- **No ball/interval arithmetic.** Unlike MPC's experimental `mpcb_t` (complex balls), `dashu`
  does not provide interval or ball types. This is a deliberate scope choice: ball arithmetic is
  still experimental upstream, and if/when it stabilizes it is better provided by a **separate
  crate** layered on `dashu-float`/`dashu-cmplx` than coupled to the core types.
- **No SIMD-FFT multiplication** yet (planned for v1.0).

## MSRV and feature policy

The current MSRV is **1.68**. Third-party integrations follow a versioned-feature convention: stable dependencies use `xxx_vYY` (e.g. `rand_v08`) with an unversioned `xxx` alias pinned to one version, while unstable dependencies alias `xxx` to the newest. See [Cargo Features](./index.md#cargo-features) for the full explanation.
