//! Shared strategies and helpers for the `fuzz` differential tests.
//!
//! The test binaries under `fuzz/tests/` are proptest-driven differentials against `rug` (GMP/MPFR/
//! MPC) or an internal exact-then-round oracle. They live in a workspace-excluded crate and are run
//! manually before a release; they are **not** part of CI's per-PR test job (CI only `cargo check`s
//! this crate — see the `fuzz-check` workflow). Proptest gives shrinking: a failing differential
//! reduces to a minimal counterexample.
//!
//! Run them with [`run.sh`](../run.sh) (`fuzz/run.sh`, `-h` for options), which gives every test —
//! and every requested *shard* of a test — its own process and keeps all cores busy; the plain
//! `cargo test --manifest-path fuzz/Cargo.toml -- --ignored` still works but serializes a long test
//! into a single thread. [`fuzz_config`] documents the `FUZZ_CASES` / `FUZZ_SHARDS` / `FUZZ_SEED`
//! knobs and [`sampled_precisions`] the per-width subsampling.

use dashu::float::ops::Abs;
use dashu::float::round::mode::HalfAway;
use dashu::float::{Context, DBig, FBig, Repr};
use dashu::integer::{IBig, UBig, Word};
use proptest::prelude::*;
use proptest::test_runner::RngSeed;

/// The shared ulp tolerance of the nearest-mode differentials — float and complex suites alike,
/// so their strictness stays comparable. Both sides of a differential are (near-)correctly
/// rounded, so each lands within ~1 ulp of the true value and the two results must agree to
/// within [`CLOSE_K`] ulps; a larger divergence is a bug in one of the two implementations.
/// (The directed, bit-exact suites don't use this constant — they assert the straddle contract
/// with no tolerance at all.)
pub const CLOSE_K: i32 = 2;

/// |dashu − rug| ≤ `k` ulps at dashu's precision (pass [`CLOSE_K`] for `k` in the differentials).
/// Exact agreement short-circuits before the ulp comparison — that also avoids `.ulp()` on
/// unlimited-precision results (e.g. `powi(x, 0) = 1`).
pub fn within_k_ulps(d: &DBig, r: &DBig, k: i32) -> bool {
    let diff = (d.clone() - r).abs();
    if diff.repr().significand().is_zero() {
        return true;
    }
    diff <= d.ulp() * k
}

/// Read an integer env var, ignoring unset/unparseable values.
fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name).ok().and_then(|s| s.trim().parse().ok())
}

/// The fuzz strength, shaped by three env knobs:
///
/// - **`FUZZ_CASES`** (fallback `PROPTEST_CASES`, default 1024) — the case budget of the whole
///   test, spent at the **lowest** configured [`fuzz_precisions_bits`] width (higher widths
///   self-subsample via [`sampled_precisions`] — half the cases each step up, quarter for the
///   widths past [`MAX_SAMPLE_SHIFT`], so the default sweep costs 2× the budget, not 4×).
///   1024 is a few seconds per differential; raise it for a thorough release pass, lower for a
///   smoke run. A budget of 0 is ignored (it would run no case and still pass).
/// - **`FUZZ_SHARDS`** / **`FUZZ_SHARD`** — process-level sharding for a test whose tail is
///   long: the budget is divided by `FUZZ_SHARDS` and shard *i* takes the *i*-th slice with
///   its own seed, so `FUZZ_SHARDS=16` across 16 processes keeps the coverage of a single
///   run while using 16 cores. Shrinking is unaffected (each shard shrinks within its own
///   slice). `FUZZ_SHARD` defaults to 0 and must be `< FUZZ_SHARDS`.
/// - **`FUZZ_SEED`** — pins the RNG (shard *i* gets `FUZZ_SEED + i`), making a sharded run
///   reproducible. Unset → proptest's entropy-based seed, so repeated runs explore new
///   inputs (the point of a fuzz suite); `run.sh` sets it for you when `FUZZ_SEED` is
///   exported. Note that proptest's own `PROPTEST_RNG_SEED` is *not* honored (the explicit
///   `rng_seed` above overrides it) — use `FUZZ_SEED`.
pub fn fuzz_config() -> ProptestConfig {
    const DEFAULT_BUDGET: usize = 1024;
    // A zero budget would run no case at all and still report success, so it falls back to the
    // default rather than being honored (`run.sh` rejects it outright). The per-shard count is
    // *clamped*, not truncated: `as u32` alone would turn an absurd budget into zero.
    let budget = match env_usize("FUZZ_CASES")
        .or_else(|| env_usize("PROPTEST_CASES"))
        .unwrap_or(DEFAULT_BUDGET)
    {
        0 => DEFAULT_BUDGET,
        budget => budget,
    };
    let shards = env_usize("FUZZ_SHARDS").unwrap_or(1).max(1);
    let shard = env_usize("FUZZ_SHARD").unwrap_or(0).min(shards - 1);
    let rng_seed = match env_usize("FUZZ_SEED") {
        Some(seed) => RngSeed::Fixed((seed as u64).wrapping_add(shard as u64)),
        None => RngSeed::Random,
    };
    ProptestConfig {
        cases: budget.div_ceil(shards).clamp(1, u32::MAX as usize) as u32,
        rng_seed,
        ..ProptestConfig::default()
    }
}

/// Precision sweep (in **bits**) for the float/complex differential tests. Defaults to
/// `[50, 100, 500, 1000]`; override with the `FUZZ_PRECISIONS` env var as comma-separated bits
/// (e.g. `FUZZ_PRECISIONS=53` for a fast single-precision pass, or `=50,100,500,1000`). Empty or
/// unparseable values fall back to the default.
pub fn fuzz_precisions_bits() -> Vec<u32> {
    const DEFAULT: [u32; 4] = [50, 100, 500, 1000];
    match std::env::var("FUZZ_PRECISIONS") {
        Ok(s) => {
            let parsed: Vec<u32> = s.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            if parsed.is_empty() {
                DEFAULT.to_vec()
            } else {
                parsed
            }
        }
        Err(_) => DEFAULT.to_vec(),
    }
}

/// The same sweep as **decimal digits** for the base-10 (`DBig`) float tests: `ceil(bits × log₁₀2)`
/// (50b→16, 100b→31, 500b→151, 1000b→301 digits). One env var (in bits) thus drives both the
/// base-2 complex tests and the base-10 float tests at a consistent underlying precision.
pub fn fuzz_precisions_decimal() -> Vec<usize> {
    fuzz_precisions_bits()
        .into_iter()
        .map(|b| ((b as f64) * core::f64::consts::LOG10_2).ceil() as usize)
        .collect()
}

/// A deterministic per-case key for [`sampled_precisions`], hashed from the case inputs'
/// `Debug` representations (`FBig`/`CBig` don't implement `Hash`). The `Debug` form is
/// deterministic (the compact head‥tail view truncates long significands, but always the
/// same way), which is all the key needs: same input, same key, within a run and across
/// runs. Two inputs differing only inside a truncated middle can collide — astronomically
/// unlikely for random draws, and the cost is only a skipped wider width.
pub fn case_key(inputs: &[&dyn core::fmt::Debug]) -> u64 {
    use core::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for input in inputs {
        format!("{input:?}").hash(&mut h);
    }
    h.finish()
}

/// The widest subsampling the sweep applies: width *i* runs on a `2^-min(i, MAX_SAMPLE_SHIFT)`
/// fraction of the cases. Uncapped, the widest width of a sweep gets `2^-(n-1)` of the budget —
/// 128 of 1024 cases for the default four widths, an 8× drop in sensitivity exactly where the
/// general multi-word kernels are exercised (the width most likely to catch a bigint bug is the
/// one sampled hardest). Capped at `2^-2`, the default sweep costs 2× the budget
/// (`N + N/2 + N/4 + N/4`) against an unsampled 4×, while its widest width keeps a quarter.
const MAX_SAMPLE_SHIFT: u32 = 2;

/// Subsample a precision sweep: the lowest configured width runs on **every** case (its
/// iteration count is the `cases` budget of [`fuzz_config`], env-tunable); width *i* runs only
/// when the low *min(i, `MAX_SAMPLE_SHIFT`)* bits of the case key are all zero, i.e. on a
/// `2^-min(i, 2)` fraction of cases — the expensive widths get proportionally fewer iterations,
/// floored at a quarter (N, N/2, N/4, N/4 for the default four widths). With a single width
/// configured the gate is fully open and every case runs at that width.
///
/// The gate depends only on the case inputs, so a counterexample reproduces bit-for-bit.
/// Shrinking can in principle change the key (a shorter input has a different `Debug`),
/// closing the width gate mid-shrink — harmless: the failing width that proptest reports is
/// the one the unshrunk case actually ran.
pub fn sampled_precisions<P>(key: u64, precs: Vec<P>) -> impl Iterator<Item = P> {
    precs.into_iter().enumerate().filter(move |(i, _)| {
        let shift = (*i as u32).min(MAX_SAMPLE_SHIFT);
        shift == 0 || key & ((1u64 << shift) - 1) == 0
    }).map(|(_, p)| p)
}

/// Convenience over [`sampled_precisions`] for the decimal-digit sweep.
pub fn sampled_precisions_decimal(key: u64) -> impl Iterator<Item = usize> {
    sampled_precisions(key, fuzz_precisions_decimal())
}

/// Convenience over [`sampled_precisions`] for the bit-width sweep.
pub fn sampled_precisions_bits(key: u64) -> impl Iterator<Item = u32> {
    sampled_precisions(key, fuzz_precisions_bits())
}

/// A random `IBig` of bounded magnitude (up to `max_words · 64` bits) with a random sign. Trailing
/// zero words are trimmed so that proptest shrinking can reduce the magnitude to a minimal failing
/// case rather than getting stuck on a large zero-padded significand.
pub fn ibig_strategy(max_words: usize) -> impl Strategy<Value = IBig> {
    (any::<bool>(), prop::collection::vec(any::<Word>(), 0..max_words)).prop_map(
        |(neg, mut words)| {
            while words.last() == Some(&0) {
                words.pop();
            }
            let mag = if words.is_empty() {
                UBig::ZERO
            } else {
                UBig::from_words(&words)
            };
            let v = IBig::from(mag);
            if neg && !v.is_zero() { -v } else { v }
        },
    )
}

/// A random `UBig` of bounded magnitude (no sign) — for unsigned integer oracles (sqrt / root /
/// bit-ops / power-of-two). Trims trailing zero words for better shrinking.
pub fn ubig_strategy(max_words: usize) -> impl Strategy<Value = UBig> {
    prop::collection::vec(any::<Word>(), 0..max_words).prop_map(|mut words| {
        while words.last() == Some(&0) {
            words.pop();
        }
        if words.is_empty() {
            UBig::ZERO
        } else {
            UBig::from_words(&words)
        }
    })
}

/// A random base-10 `DBig` (= `FBig<HalfAway, 10>`) at unlimited precision, exponent drawn from
/// `exp_range`. Each test re-rounds it to a target precision via its own `Context`.
pub fn dbig_strategy(
    exp_range: std::ops::RangeInclusive<isize>,
) -> impl Strategy<Value = FBig<HalfAway, 10>> {
    (ibig_strategy(5), exp_range).prop_map(|(sig, exp)| {
        FBig::from_repr(Repr::<10>::new(sig, exp), Context::<HalfAway>::new(0))
    })
}

/// A positive base-10 `DBig` at unlimited precision (significand ≥ 1), for the ln/sqrt/powf domains.
pub fn pos_dbig_strategy(
    exp_range: std::ops::RangeInclusive<isize>,
) -> impl Strategy<Value = FBig<HalfAway, 10>> {
    (prop::collection::vec(any::<Word>(), 1..5), exp_range).prop_map(|(mut words, exp)| {
        while words.last() == Some(&0) {
            words.pop();
        }
        if words.is_empty() {
            words.push(1);
        }
        FBig::from_repr(
            Repr::<10>::new(IBig::from(UBig::from_words(&words)), exp),
            Context::<HalfAway>::new(0),
        )
    })
}

/// A base-10 `DBig` in `[-1, 1]` (as `n/1000`), for the real `asin`/`acos`/`atanh`/`ln_1p` domains.
/// Shrinks toward 0.
pub fn unit_dbig() -> impl Strategy<Value = FBig<HalfAway, 10>> {
    (-1000i32..=1000)
        .prop_map(|n| FBig::from_repr(Repr::<10>::new(n.into(), -3), Context::<HalfAway>::new(0)))
}

/// Shared helpers for the `CBig` vs `rug::Complex` (MPC) differentials, run across the
/// [`fuzz_precisions_bits`](crate::fuzz_precisions_bits) sweep.
pub mod cmplx {
    use core::convert::TryFrom;
    use crate::CLOSE_K;
    use dashu::complex::CBig;
    use dashu::float::FBig;
    use dashu::float::round::mode::HalfEven;
    use dashu::float::round::Round;
    use proptest::prelude::*;
    use rug::ops::Pow;

    pub type C = CBig<HalfEven, 2>;
    pub type F = FBig<HalfEven, 2>;

    /// A modest-magnitude finite `f64` (`±(1..=8) · [1,2) · 2^(-2..=2)`), shrinking toward small values.
    pub fn f64_part() -> impl Strategy<Value = f64> {
        (1u8..=8, any::<bool>(), 0u32..1000, -2i32..=2).prop_map(|(sig, neg, frac, exp)| {
            let mant = 1.0 + (frac as f64) / 1000.0;
            let mag = (sig as f64) * mant * 2f64.powi(exp);
            if neg { -mag } else { mag }
        })
    }

    /// Build a base-2 `FBig` from an `f64` at `prec` bits.
    pub fn fbig_from(v: f64, prec: usize) -> F {
        F::try_from(v).unwrap().with_precision(prec).value()
    }

    /// Build a dashu `CBig` and a matching `rug::Complex`, both at `prec` bits, from `f64` parts.
    /// (`CBig::from_parts` takes `Context::max` of its parts, so two `prec`-precision parts yield a
    /// `prec`-precision `CBig` whose `z.context().<op>()` computes at `prec`.)
    pub fn pair(re: f64, im: f64, prec: usize) -> (C, rug::Complex) {
        let cbig = CBig::from_parts(fbig_from(re, prec), fbig_from(im, prec));
        let rug = rug::Complex::with_val(prec as u32, (re, im));
        (cbig, rug)
    }

    /// Exact conversion of a base-2 `FBig` to a `rug::Float` at `cmp_bits`. The value is
    /// `significand × 2^exponent`, so build the significand (IBig → decimal string → `rug::Integer`,
    /// sign preserved) and scale by `2^exp`. Exact because `cmp_bits` exceeds the significand's
    /// bit length (which is ≤ the working precision + 1 guard).
    fn fbig2_to_rug<R: Round>(f: &FBig<R, 2>, cmp_bits: u32) -> rug::Float {
        let repr = f.repr();
        let sig = repr.significand();
        let exp = repr.exponent();
        if sig.is_zero() {
            return rug::Float::new(cmp_bits);
        }
        let int = sig.to_string().parse::<rug::Integer>().unwrap();
        let mag = rug::Float::with_val(cmp_bits, &int);
        let two = rug::Float::with_val(cmp_bits, 2u32);
        if exp >= 0 {
            mag * two.pow(exp as u32)
        } else {
            mag / two.pow((-exp) as u32)
        }
    }

    /// True when both the dashu result and the rug reference are finite — use to skip a
    /// (precision, input) pair (overflow / branch-point blow-up) rather than fail on it.
    pub fn complex_finite(d: &C, r: &rug::Complex) -> bool {
        d.is_finite() && r.real().is_finite() && r.imag().is_finite()
    }

    /// Precision-aware agreement: convert `d` to rug at `2·prec + 64` bits and check each component
    /// is within `CLOSE_K × 2^-prec × scale` of the reference, where `scale` is the largest component
    /// magnitude of either side. Returns `false` if either side is non-finite (caller skips).
    ///
    /// With [`CLOSE_K`] = 2 the budget is one ulp per side, so the caller must evaluate the
    /// reference **above** `prec` (see [`ref_bits`]) on the same input — otherwise MPC's own
    /// prec-bit rounding eats the budget and a mismatch indicts MPC, not us.
    pub fn close_at(d: &C, r: &rug::Complex, prec: usize) -> bool {
        let cmp = (2 * prec + 64) as u32;
        let (dre, dim) = d.clone().into_parts();
        let dre_r = fbig2_to_rug(&dre, cmp);
        let dim_r = fbig2_to_rug(&dim, cmp);
        let rre = rug::Float::with_val(cmp, r.real());
        let rim = rug::Float::with_val(cmp, r.imag());

        if !dre_r.is_finite() || !dim_r.is_finite() || !rre.is_finite() || !rim.is_finite() {
            return false;
        }

        // Scale by the largest component magnitude (f64 is plenty for a magnitude reference on the
        // bounded results these tests produce).
        let scale_f64 = dre_r
            .to_f64()
            .abs()
            .max(dim_r.to_f64().abs())
            .max(rre.to_f64().abs())
            .max(rim.to_f64().abs())
            .max(1e-300);
        let scale = rug::Float::with_val(cmp, scale_f64);
        let two_pow_prec = rug::Float::with_val(cmp, 2u32).pow(prec as u32);
        let allowed = rug::Float::with_val(cmp, CLOSE_K) * &scale / &two_pow_prec;

        let re_err = (dre_r - &rre).abs();
        let im_err = (dim_r - &rim).abs();
        re_err <= allowed.clone() && im_err <= allowed
    }

    // ========================================================================
    // Directed rounding — bit-exact per component (the float directed contract)
    // ========================================================================

    /// The reference precision for the directed tests: the MPC reference runs **nearest** at
    /// `2·prec + 512` bits, so its own error (~`2^-(2prec+512)`) sits ~`2^(prec+512)` below the
    /// target-precision rounding boundaries (~`2^-prec`) — re-rounding the reference can only
    /// mis-decide a true value within `2^-(prec+512)` of a boundary. The same margin argument
    /// backs the float directed harness.
    pub fn ref_bits(prec: u32) -> u32 {
        2 * prec + 512
    }

    /// A mode-`R` base-2 `FBig` part from an `f64` at `prec` bits.
    pub fn part<R: Round>(v: f64, prec: u32) -> FBig<R, 2> {
        FBig::<R, 2>::try_from(v).unwrap().with_precision(prec as usize).value()
    }

    /// Directed per-component check: dashu's mode-`R` result must equal the **Up- or the
    /// Down-rounding** of the high-precision nearest reference (when the two agree, the true
    /// value was representable and every mode must return it — the same straddle contract as
    /// the float directed tests). Both sides convert exactly to `rug::Float`
    /// ([`fbig2_to_rug`]), so the comparison is bit-exact. Value equality reads ±0 as equal —
    /// sign-of-zero is covered by the signed-zero tables instead.
    pub fn directed_eq_part<R: Round>(d: &FBig<R, 2>, hi: &rug::Float, prec: u32) -> bool {
        let cmp = 2 * prec + 64;
        let d_r = fbig2_to_rug(d, cmp);
        let (up, _) = rug::Float::with_val_round(prec, hi, rug::float::Round::Up);
        let (down, _) = rug::Float::with_val_round(prec, hi, rug::float::Round::Down);
        d_r == up || (up != down && d_r == down)
    }

    /// Directed check for a whole complex result — both components independently (dashu's
    /// `CBig` rounds each part with the single mode `R`, so the reference must too).
    pub fn directed_eq<R: Round>(d: &CBig<R, 2>, hi: &rug::Complex, prec: u32) -> bool {
        let (dre, dim) = d.clone().into_parts();
        directed_eq_part(&dre, hi.real(), prec) && directed_eq_part(&dim, hi.imag(), prec)
    }

}
