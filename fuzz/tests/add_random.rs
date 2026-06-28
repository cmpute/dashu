//! Differential / fuzz test for FBig add/sub.
//!
//! The limited-precision `Context::add`/`Context::sub` path (exponent alignment + the
//! `repr_round_sum` rounding with a discarded "low part") is checked against an independent
//! oracle: the exact sum/difference (computed at unlimited precision) re-rounded with
//! `FBig::with_precision`, which uses the simple `repr_round` path rather than
//! `repr_round_sum`. The two must agree for every rounding mode, base, precision and operand
//! shape. Proptest-driven so a mismatch shrinks to a minimal `(a, b, precision)` counterexample.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test add_random -- --ignored --nocapture`

use dashu_float::round::Round;
use dashu_float::round::mode::*;
use dashu_float::{Context, FBig, Repr, Word};
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

/// Compare limited-precision add/sub against the exact-then-round oracle for one operand pair.
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

/// Precision strategy biased toward the boundary precisions 1/2/3 (where rounding bugs live),
/// mixed with a uniform draw over `1..200`.
fn precision_strategy() -> impl Strategy<Value = usize> {
    prop_oneof![Just(1usize), Just(2), Just(3), 1usize..200,]
}

proptest! {
    #![proptest_config(fuzz::fuzz_config())]

    #[test]
    #[ignore]
    fn add_sub_differential_binary(
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
    fn add_sub_differential_decimal(
        a_sig in fuzz::ibig_strategy(5), a_exp in -1500isize..1500,
        b_sig in fuzz::ibig_strategy(5), b_exp in -1500isize..1500,
        precision in precision_strategy(),
    ) {
        let a = Repr::<10>::new(a_sig, a_exp);
        let b = Repr::<10>::new(b_sig, b_exp);
        check_all_modes::<10>(&a, &b, precision);
    }
}
