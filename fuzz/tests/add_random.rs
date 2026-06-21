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

use dashu_float::round::Round;
use dashu_float::round::mode::*;
use dashu_float::{Context, FBig, Repr, Word};
use dashu_int::{rand_v010::UniformBits, IBig};
use rand::prelude::*;

/// Random signed significand drawn directly as a random `IBig` of bounded bit length.
///
/// The magnitude is at most ~266 bits (≈ 80 decimal digits), matching the coverage of the
/// previous decimal-string generator but without per-iteration allocation + parsing. A
/// base-agnostic integer lets the same significand feed `Repr::<B>` for any base `B`.
fn random_significand<R: Rng + ?Sized>(rng: &mut R) -> IBig {
    let bits = rng.random_range(1..=266usize);
    rng.sample(UniformBits::new(bits))
}

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

    let actual_add = ctx.add(a, b).value().repr().clone();
    let (add_p, add_p1) =
        rounded_oracle::<R, B>(unlimited.add(a, b).value().repr().clone(), precision);
    assert!(
        actual_add == add_p || actual_add == add_p1,
        "add mismatch (mode={mode_name}, p={precision})\n a={a:?}\n b={b:?}\n actual={actual_add:?}\n oracle(p)={add_p:?}\n oracle(p+1)={add_p1:?}",
    );

    let actual_sub = ctx.sub(a, b).value().repr().clone();
    let (sub_p, sub_p1) =
        rounded_oracle::<R, B>(unlimited.sub(a, b).value().repr().clone(), precision);
    assert!(
        actual_sub == sub_p || actual_sub == sub_p1,
        "sub mismatch (mode={mode_name}, p={precision})\n a={a:?}\n b={b:?}\n actual={actual_sub:?}\n oracle(p)={sub_p:?}\n oracle(p+1)={sub_p1:?}",
    );
}

fn run_mode<R: Round, const B: Word>(rng: &mut StdRng, iters: usize, mode_name: &str) {
    // Deterministic small-precision sweep: precisions 1, 2, 3 are where rounding bugs live,
    // but the random `1..200` below hits them only by chance. Always exercise these boundary
    // precisions (ported from the removed `add_sub_oracle` example).
    for &precision in &[1usize, 2, 3, 5, 10] {
        for _ in 0..50 {
            let a_exp = rng.random_range(-1500..1500) as isize;
            let a = Repr::<B>::new(random_significand(rng), a_exp);
            let b_exp = rng.random_range(-1500..1500) as isize;
            let b = Repr::<B>::new(random_significand(rng), b_exp);
            check_pair::<R, B>(&a, &b, precision, mode_name);
        }
    }

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
