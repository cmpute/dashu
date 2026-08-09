//! Differential / fuzz tests for FBig arithmetic (add/sub/mul/div/sqr/cubic) at limited precision.
//!
//! add/sub/mul/sqr/cubic are checked against an EXACT-then-round oracle: the exact result at
//! unlimited precision (sum, difference, product, square, or cube — all finite) re-rounded with
//! `FBig::with_precision`,
//! which uses the simple `repr_round` path rather than the limited-precision `repr_round_sum` /
//! product-rounding paths. The two must agree for every rounding mode, base, precision, operand.
//!
//! div has no finite exact form (quotients are generally non-terminating), so it is checked
//! against a high-precision quotient (`precision + 50` guard digits/bits) re-rounded to the target
//! — the guard makes a real divergence a bug, with a 2-ulp tolerance for the rare rounding-boundary
//! case (mirroring the transcendental differentials). A zero divisor is skipped.
//!
//! Proptest-driven so a mismatch shrinks to a minimal `(a, b, precision)` counterexample.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test float_random -- --ignored --nocapture`

use dashu::base::Sign;
use dashu::float::ops::Abs;
use dashu::float::round::Round;
use dashu::float::round::mode::*;
use dashu::float::{Context, FBig, Repr, Word};
use proptest::prelude::*;

/// Round the exact result to `precision` and `precision + 1` digits, returning both.
///
/// The library may carry one guard digit on an inexact effective subtraction (see AGENTS.md),
/// so a correct limited-precision result matches *either* rounding — a plain addition, or an
/// identity like `x + 0`, rounds to `precision`; a genuine inexact subtraction rounds to
/// `precision + 1`. Comparing against both avoids relying on a fragile sign-based `is_sub`
/// heuristic (which misfires, for example, when one operand is zero).
fn rounded_oracle<R: Round, const B: Word>(exact: Repr<B>, precision: usize) -> (Repr<B>, Repr<B>) {
    // Attach a precision larger than the value's digit count so `with_precision` actually
    // rounds (it only rounds when the source precision exceeds the target).
    let d = exact.digits().max(precision + 2).max(1);
    let rp = FBig::<R, B>::from_repr(exact.clone(), Context::<R>::new(d))
        .with_precision(precision)
        .value()
        .repr()
        .clone();
    let rp1 = FBig::<R, B>::from_repr(exact, Context::<R>::new(d))
        .with_precision(precision + 1)
        .value()
        .repr()
        .clone();
    (rp, rp1)
}

/// |a - b| ≤ `k` ulps of `a` (for div's high-precision oracle; mirrors the helper in the
/// transcendental differentials).
fn within_k_ulps<R: Round, const B: Word>(a: &FBig<R, B>, b: &FBig<R, B>, k: i32) -> bool {
    let diff = (a.clone() - b).abs();
    if diff.repr().significand().is_zero() {
        return true;
    }
    diff <= a.ulp() * k
}

/// Compare limited-precision add/sub/mul/div against their oracles for one operand pair + mode.
///
/// add/sub/mul use the exact-then-round oracle (`rounded_oracle`); div uses a high-precision
/// quotient re-rounded to `precision` (`within_k_ulps`), with a zero divisor skipped.
fn check_pair<R: Round, const B: Word>(
    a: &Repr<B>,
    b: &Repr<B>,
    precision: usize,
    mode_name: &str,
) {
    let ctx = Context::<R>::new(precision);
    let unlimited = Context::<R>::new(0);

    let actual_add = ctx.add(a, b).unwrap().value().repr().clone();
    let (add_p, add_p1) =
        rounded_oracle::<R, B>(unlimited.add(a, b).unwrap().value().repr().clone(), precision);
    assert!(
        actual_add == add_p || actual_add == add_p1,
        "add mismatch (mode={mode_name}, p={precision})\n a={a:?}\n b={b:?}\n actual={actual_add:?}\n oracle(p)={add_p:?}\n oracle(p+1)={add_p1:?}",
    );

    let actual_sub = ctx.sub(a, b).unwrap().value().repr().clone();
    let (sub_p, sub_p1) =
        rounded_oracle::<R, B>(unlimited.sub(a, b).unwrap().value().repr().clone(), precision);
    assert!(
        actual_sub == sub_p || actual_sub == sub_p1,
        "sub mismatch (mode={mode_name}, p={precision})\n a={a:?}\n b={b:?}\n actual={actual_sub:?}\n oracle(p)={sub_p:?}\n oracle(p+1)={sub_p1:?}",
    );

    // mul: exact product, sharing the exact-then-round oracle with add/sub.
    let actual_mul = ctx.mul(a, b).unwrap().value().repr().clone();
    let (mul_p, mul_p1) =
        rounded_oracle::<R, B>(unlimited.mul(a, b).unwrap().value().repr().clone(), precision);
    assert!(
        actual_mul == mul_p || actual_mul == mul_p1,
        "mul mismatch (mode={mode_name}, p={precision})\n a={a:?}\n b={b:?}\n actual={actual_mul:?}\n oracle(p)={mul_p:?}\n oracle(p+1)={mul_p1:?}",
    );

    // sqr / cubic: exact square / cube of `a` (single operand), sharing the exact-then-round
    // oracle with add/sub/mul.
    let actual_sqr = ctx.sqr(a).unwrap().value().repr().clone();
    let (sqr_p, sqr_p1) =
        rounded_oracle::<R, B>(unlimited.sqr(a).unwrap().value().repr().clone(), precision);
    assert!(
        actual_sqr == sqr_p || actual_sqr == sqr_p1,
        "sqr mismatch (mode={mode_name}, p={precision})\n a={a:?}\n actual={actual_sqr:?}\n oracle(p)={sqr_p:?}\n oracle(p+1)={sqr_p1:?}",
    );

    let actual_cub = ctx.cubic(a).unwrap().value().repr().clone();
    let (cub_p, cub_p1) =
        rounded_oracle::<R, B>(unlimited.cubic(a).unwrap().value().repr().clone(), precision);
    assert!(
        actual_cub == cub_p || actual_cub == cub_p1,
        "cubic mismatch (mode={mode_name}, p={precision})\n a={a:?}\n actual={actual_cub:?}\n oracle(p)={cub_p:?}\n oracle(p+1)={cub_p1:?}",
    );

    // div: no finite exact quotient — compare against a high-precision quotient re-rounded to
    // `precision`. Skip a zero divisor (div-by-zero errors).
    if !b.significand().is_zero() {
        let actual_div = ctx.div(a, b).unwrap().value();
        let high = Context::<R>::new(precision + 50)
            .div(a, b)
            .unwrap()
            .value()
            .with_precision(precision)
            .value();
        assert!(
            within_k_ulps(&actual_div, &high, 2),
            "div mismatch (mode={mode_name}, p={precision})\n a={a:?}\n b={b:?}\n actual={actual_div:?}\n high-prec rounded={high:?}",
        );
    }
}

/// Run `check_pair` under all six rounding modes for one operand pair + precision.
fn check_all_modes<const B: Word>(a: &Repr<B>, b: &Repr<B>, precision: usize) {
    check_pair::<Zero, B>(a, b, precision, "Zero");
    check_pair::<Away, B>(a, b, precision, "Away");
    check_pair::<Up, B>(a, b, precision, "Up");
    check_pair::<Down, B>(a, b, precision, "Down");
    check_pair::<HalfEven, B>(a, b, precision, "HalfEven");
    check_pair::<HalfAway, B>(a, b, precision, "HalfAway");
}

/// Compare limited-precision `fma(a, b, c, sign) = round(c + sign·(a·b))` — a single fused rounding —
/// against the exact `c ± a·b` computed at unlimited precision and re-rounded (sharing
/// `rounded_oracle` with add/sub/mul).
fn check_fma<R: Round, const B: Word>(
    a: &Repr<B>,
    b: &Repr<B>,
    c: &Repr<B>,
    sign: Sign,
    precision: usize,
    mode_name: &str,
) {
    let ctx = Context::<R>::new(precision);
    let unlimited = Context::<R>::new(0);
    let ab = unlimited.mul(a, b).unwrap().value().repr().clone();
    let exact = match sign {
        Sign::Positive => unlimited.add(c, &ab).unwrap().value().repr().clone(),
        Sign::Negative => unlimited.sub(c, &ab).unwrap().value().repr().clone(),
    };
    let actual = ctx.fma(a, b, c, sign).unwrap().value().repr().clone();
    let (fma_p, fma_p1) = rounded_oracle::<R, B>(exact, precision);
    assert!(
        actual == fma_p || actual == fma_p1,
        "fma mismatch (mode={mode_name}, p={precision})\n a={a:?}\n b={b:?}\n c={c:?}\n sign={sign:?}\n actual={actual:?}\n oracle(p)={fma_p:?}\n oracle(p+1)={fma_p1:?}",
    );
}

/// `fma` under all six rounding modes, both signs of the `±z3` addend.
fn check_fma_all_modes<const B: Word>(a: &Repr<B>, b: &Repr<B>, c: &Repr<B>, precision: usize) {
    check_fma::<Zero, B>(a, b, c, Sign::Positive, precision, "Zero+");
    check_fma::<Zero, B>(a, b, c, Sign::Negative, precision, "Zero-");
    check_fma::<Away, B>(a, b, c, Sign::Positive, precision, "Away+");
    check_fma::<Away, B>(a, b, c, Sign::Negative, precision, "Away-");
    check_fma::<Up, B>(a, b, c, Sign::Positive, precision, "Up+");
    check_fma::<Up, B>(a, b, c, Sign::Negative, precision, "Up-");
    check_fma::<Down, B>(a, b, c, Sign::Positive, precision, "Down+");
    check_fma::<Down, B>(a, b, c, Sign::Negative, precision, "Down-");
    check_fma::<HalfEven, B>(a, b, c, Sign::Positive, precision, "HalfEven+");
    check_fma::<HalfEven, B>(a, b, c, Sign::Negative, precision, "HalfEven-");
    check_fma::<HalfAway, B>(a, b, c, Sign::Positive, precision, "HalfAway+");
    check_fma::<HalfAway, B>(a, b, c, Sign::Negative, precision, "HalfAway-");
}

/// Precision strategy biased toward the boundary precisions 1/2/3 (where rounding bugs live),
/// mixed with a uniform draw over `1..200`.
fn precision_strategy() -> impl Strategy<Value = usize> {
    prop_oneof![Just(1usize), Just(2), Just(3), 1usize..200,]
}

proptest! {
    #![proptest_config(fuzz::fuzz_config())]

    #[test]
    #[ignore]
    fn fbig_arithmetic_binary_fuzz(
        a_sig in fuzz::ibig_strategy(5), a_exp in -1500isize..1500,
        b_sig in fuzz::ibig_strategy(5), b_exp in -1500isize..1500,
        precision in precision_strategy(),
    ) {
        let a = Repr::<2>::new(a_sig, a_exp);
        let b = Repr::<2>::new(b_sig, b_exp);
        check_all_modes::<2>(&a, &b, precision);
    }

    #[test]
    #[ignore]
    fn fbig_arithmetic_decimal_fuzz(
        a_sig in fuzz::ibig_strategy(5), a_exp in -1500isize..1500,
        b_sig in fuzz::ibig_strategy(5), b_exp in -1500isize..1500,
        precision in precision_strategy(),
    ) {
        let a = Repr::<10>::new(a_sig, a_exp);
        let b = Repr::<10>::new(b_sig, b_exp);
        check_all_modes::<10>(&a, &b, precision);
    }

    /// fma (fused multiply-add, `c + sign·(a·b)`) under all modes and both signs, in bases 2 and 10.
    #[test]
    #[ignore]
    fn fbig_fma_fuzz(
        a_sig in fuzz::ibig_strategy(5), a_exp in -1500isize..1500,
        b_sig in fuzz::ibig_strategy(5), b_exp in -1500isize..1500,
        c_sig in fuzz::ibig_strategy(5), c_exp in -1500isize..1500,
        precision in precision_strategy(),
    ) {
        check_fma_all_modes::<2>(
            &Repr::<2>::new(a_sig.clone(), a_exp),
            &Repr::<2>::new(b_sig.clone(), b_exp),
            &Repr::<2>::new(c_sig.clone(), c_exp),
            precision,
        );
        check_fma_all_modes::<10>(
            &Repr::<10>::new(a_sig, a_exp),
            &Repr::<10>::new(b_sig, b_exp),
            &Repr::<10>::new(c_sig, c_exp),
            precision,
        );
    }
}
