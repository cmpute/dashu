//! Shared strategies and helpers for the `fuzz` differential tests.
//!
//! The test binaries under `fuzz/tests/` are proptest-driven differentials against `rug` (GMP/MPFR/
//! MPC) or an internal exact-then-round oracle. They live in a workspace-excluded crate and are run
//! manually before a release (`cargo test --manifest-path fuzz/Cargo.toml -- --ignored`); they are
//! **not** part of CI's per-PR test job (CI only `cargo check`s this crate — see the `fuzz-check`
//! workflow). Proptest gives shrinking: a failing differential reduces to a minimal counterexample.

use dashu::float::round::mode::HalfAway;
use dashu::float::{Context, FBig, Repr};
use dashu::integer::{IBig, UBig, Word};
use proptest::prelude::*;

/// Default fuzz strength — more cases than CI's per-crate `PROPTEST_CASES=256`, since these run
/// out-of-band and are meant to be thorough. Overridable via the `PROPTEST_CASES` env var.
pub fn fuzz_config() -> ProptestConfig {
    ProptestConfig {
        cases: 1024,
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
    use dashu::complex::CBig;
    use dashu::float::FBig;
    use dashu::float::round::mode::HalfEven;
    use proptest::prelude::*;
    use rug::ops::Pow;

    pub type C = CBig<HalfEven, 2>;
    pub type F = FBig<HalfEven, 2>;

    /// Per-component ulp tolerance for the differential: results must agree to within `CLOSE_K`
    /// ulps at the working precision. Near-correctly-rounded results (both dashu and MPC) pass
    /// comfortably; a gross error fails. (~500× tighter than the previous f64 `1e-12` check at
    /// 53 bits, and meaningful at any precision since it scales as `2^-prec`.)
    const CLOSE_K: u32 = 16;

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
    fn fbig2_to_rug(f: &F, cmp_bits: u32) -> rug::Float {
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
}
