//! Shared strategies and config for the `fuzz` differential tests.
//!
//! The test binaries under `fuzz/tests/` are proptest-driven differentials against `rug` (GMP/MPFR/
//! MPC) or an internal exact-then-round oracle. They live in a workspace-excluded crate and are run
//! manually before a release (`cargo test --manifest-path fuzz/Cargo.toml -- --ignored`); they are
//! **not** part of CI's per-PR test job (CI only `cargo check`s this crate — see the `fuzz-check`
//! workflow). Proptest gives shrinking: a failing differential reduces to a minimal counterexample.

use dashu_float::round::mode::HalfAway;
use dashu_float::{Context, FBig, Repr};
use dashu_int::{IBig, UBig, Word};
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

/// A random base-10 `DBig` (= `FBig<HalfAway, 10>`) at unlimited precision, exponent drawn from
/// `exp_range`. Each test re-rounds it to a target precision via its own `Context`.
pub fn dbig_strategy(
    exp_range: std::ops::RangeInclusive<isize>,
) -> impl Strategy<Value = FBig<HalfAway, 10>> {
    (ibig_strategy(5), exp_range).prop_map(|(sig, exp)| {
        FBig::from_repr(Repr::<10>::new(sig, exp), Context::<HalfAway>::new(0))
    })
}
