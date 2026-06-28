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

/// Shared helpers for the `CBig` vs `rug::Complex` (MPC) differentials at 53-bit precision.
pub mod cmplx {
    use core::convert::TryFrom;
    use dashu::complex::CBig;
    use dashu::float::FBig;
    use dashu::float::round::mode::HalfEven;
    use proptest::prelude::*;

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

    pub fn fbig_from(v: f64) -> F {
        F::try_from(v).unwrap().with_precision(53).value()
    }

    /// Build a dashu `CBig` and a matching `rug::Complex` (53-bit) from `f64` parts.
    pub fn pair(re: f64, im: f64) -> (C, rug::Complex) {
        let cbig = CBig::from_parts(fbig_from(re), fbig_from(im));
        let rug = rug::Complex::with_val(53, (re, im));
        (cbig, rug)
    }

    pub fn cbig_to_f64(z: &C) -> (f64, f64) {
        let (re, im) = z.clone().into_parts();
        (re.to_f64().value(), im.to_f64().value())
    }

    pub fn rug_to_f64(z: &rug::Complex) -> (f64, f64) {
        (z.real().to_f64(), z.imag().to_f64())
    }

    /// True when both `(re, im)` pairs are finite and agree to within a few ulps (scale-relative).
    pub fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        let (ar, ai) = a;
        let (br, bi) = b;
        if !ar.is_finite() || !ai.is_finite() || !br.is_finite() || !bi.is_finite() {
            return false; // skip non-finite (overflow / branch-point) results
        }
        let scale = ar
            .abs()
            .max(ai.abs())
            .max(br.abs())
            .max(bi.abs())
            .max(1e-300);
        let tol = scale * 1e-12;
        (ar - br).abs() <= tol && (ai - bi).abs() <= tol
    }
}
