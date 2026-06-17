//! Differential / fuzz test for FBig add/sub.
//!
//! The limited-precision `Context::add`/`Context::sub` path (exponent alignment + the
//! `repr_round_sum` rounding with a discarded "low part") is checked against an independent
//! oracle: the exact sum/difference (computed at unlimited precision) re-rounded with
//! `FBig::with_precision`, which uses the simple `repr_round` path rather than
//! `repr_round_sum`. The two must agree for every rounding mode, base, precision and operand
//! shape.
//!
//! Run with: `cargo test --manifest-path fuzz/Cargo.toml --test add_random -- --ignored --nocapture`

use core::str::FromStr;
use dashu_float::round::Round;
use dashu_float::round::mode::*;
use dashu_float::{Context, FBig, Repr, Word};
use dashu_int::IBig;
use rand::prelude::*;

/// Random signed significand, generated as a random decimal-digit string parsed into an
/// `IBig`. Using a base-agnostic integer (not a base-`B` literal) lets the same significand
/// feed `Repr::<B>` for any base `B`.
fn random_significand<R: Rng + ?Sized>(rng: &mut R) -> IBig {
    let neg = rng.random_bool(0.5);
    let n_digits = rng.random_range(1..=80);
    let mut s = String::new();
    if neg {
        s.push('-');
    }
    for _ in 0..n_digits {
        s.push(char::from_digit(rng.random_range(0..10), 10).unwrap());
    }
    IBig::from_str(&s).unwrap_or(IBig::ZERO)
}

/// Expected result of `Context::{add,sub}` at `precision`, matching the library's design.
///
/// An *exact* result is kept verbatim when it fits within the `precision + is_sub` digit
/// allowance (e.g. `101 - 0.2 = 100.8` is kept as 4 digits at precision 3; `is_sub` is true
/// there). Any longer (inexact) result is rounded to `precision`.
fn expected<R: Round, const B: Word>(
    value: FBig<R, B>,
    precision: usize,
    is_sub: bool,
) -> FBig<R, B> {
    let digits = value.repr().digits();
    let allowance = precision + is_sub as usize;
    if digits <= allowance {
        // exact, fits the allowance: pin to a finite precision without altering the value
        value.with_precision(digits.max(1)).value()
    } else {
        // too long: round down to `precision` via the simple `repr_round` path
        value
            .with_precision(digits)
            .value()
            .with_precision(precision)
            .value()
    }
}

/// Compare limited-precision add/sub against the exact-then-round oracle for one operand pair.
fn check_pair<R: Round, const B: Word>(
    a: &Repr<B>,
    b: &Repr<B>,
    precision: usize,
    mode_name: &str,
) {
    let ctx = Context::<R>::new(precision);

    let ua = FBig::<R, B>::from_repr(a.clone(), Context::<R>::new(0));
    let ub = FBig::<R, B>::from_repr(b.clone(), Context::<R>::new(0));

    // `is_sub`: the two summands of the operation have opposite signs.
    //   a + b  -> summands a, b   -> opposite iff a.sign != b.sign
    //   a - b  -> summands a, -b  -> opposite iff a.sign == b.sign
    let add_is_sub = a.sign() != b.sign();
    let sub_is_sub = a.sign() == b.sign();

    let actual_add = ctx.add(a, b).value();
    let oracle_add = expected(ua.clone() + ub.clone(), precision, add_is_sub);
    assert_eq!(
        actual_add.repr(),
        oracle_add.repr(),
        "add mismatch (mode={mode_name}, p={precision})\n a={a:?}\n b={b:?}\n actual={actual_add:?}\n oracle={oracle_add:?}",
    );

    let actual_sub = ctx.sub(a, b).value();
    let oracle_sub = expected(ua - ub, precision, sub_is_sub);
    assert_eq!(
        actual_sub.repr(),
        oracle_sub.repr(),
        "sub mismatch (mode={mode_name}, p={precision})\n a={a:?}\n b={b:?}\n actual={actual_sub:?}\n oracle={oracle_sub:?}",
    );
}

fn run_mode<R: Round, const B: Word>(rng: &mut StdRng, iters: usize, mode_name: &str) {
    for _ in 0..iters {
        // Wide exponent range so that the negligible-small, tight-align, cancellation and borrow
        // (result just below a round number) branches are all exercised.
        let a_exp = rng.random_range(-1500..1500) as isize;
        let a_sig = random_significand(rng);
        let a = Repr::<B>::new(a_sig, a_exp);

        let b_exp = rng.random_range(-1500..1500) as isize;
        let b_sig = random_significand(rng);
        let b = Repr::<B>::new(b_sig, b_exp);

        let precision = rng.random_range(1..200);
        check_pair::<R, B>(&a, &b, precision, mode_name);
    }
}

fn run_all_modes<const B: Word>(rng: &mut StdRng, iters: usize) {
    run_mode::<Zero, B>(rng, iters, "Zero");
    run_mode::<Away, B>(rng, iters, "Away");
    run_mode::<Up, B>(rng, iters, "Up");
    run_mode::<Down, B>(rng, iters, "Down");
    run_mode::<HalfEven, B>(rng, iters, "HalfEven");
    run_mode::<HalfAway, B>(rng, iters, "HalfAway");
}

#[test]
#[ignore]
fn test_add_sub_differential_binary() {
    let mut rng = StdRng::seed_from_u64(0x1234_5678_9abc_def0);
    run_all_modes::<2>(&mut rng, 10000);
}

#[test]
#[ignore]
fn test_add_sub_differential_decimal() {
    let mut rng = StdRng::seed_from_u64(0x0fed_cba9_8765_4321);
    run_all_modes::<10>(&mut rng, 10000);
}
