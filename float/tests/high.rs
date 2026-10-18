//! Fixed boundary and regression tests for short-product and short-quotient paths.
use dashu_base::{
    Approximation::{Exact, Inexact},
    Sign,
    Sign::{Negative, Positive},
};
use dashu_float::{
    round::{mode, Round},
    Context, FBig, FpError, Repr, Word,
};
use dashu_int::IBig;
mod helpers;
use helpers::{
    check_cubic, check_mul, check_products, check_sqr, quotient_oracle, scale, significand,
    words_for_precision,
};

/// Fixed dense operand pattern (no generator state): the i-th word is
/// `(i+1)·step + i`, wrapping — distinct, carry-rich words, identical on
/// every run (and sharing their low bits across word sizes, since the
/// arithmetic is modular). The top word is kept nonzero so the operand
/// really spans `len` words.
fn pattern_words(len: usize, step: u64) -> Vec<Word> {
    (0..len)
        .map(|i| {
            let w = (i as u64 + 1).wrapping_mul(step).wrapping_add(i as u64) as Word;
            if i + 1 == len && w == 0 {
                1
            } else {
                w
            }
        })
        .collect()
}

/// Exact results and midpoint ties must be resolved by the exact path.
fn check_div_boundaries<R: Round>() {
    let p = 2400;
    let ctx = Context::<R>::new(p);
    let divisor = significand(&pattern_words(words_for_precision(p, 2), 0xfeed));
    let b = Repr::<2>::new(divisor.clone(), 0);
    let half = Repr::<2>::new(divisor.clone(), -1);
    for (a, expected) in [
        (&b, Repr::<2>::one()),
        (&half, Repr::<2>::new(IBig::ONE, -1)),
    ] {
        assert_eq!(ctx.div(a, &b).unwrap().map(FBig::into_repr), Exact(expected));
    }

    // 1 + 2^-p is exactly halfway between adjacent p-bit values.
    let midpoint = Repr::<2>::new((IBig::ONE << p) + 1, -(p as isize));
    let a = Repr::<2>::new(&divisor * midpoint.significand(), midpoint.exponent());
    assert_eq!(
        ctx.div(&a, &b).unwrap().map(FBig::into_repr),
        quotient_oracle::<R, 2>(p, &a, &b)
    );
}

#[test]
fn test_div_exact_and_midpoint_results() {
    check_div_boundaries::<mode::Zero>();
    check_div_boundaries::<mode::Away>();
    check_div_boundaries::<mode::Up>();
    check_div_boundaries::<mode::Down>();
    check_div_boundaries::<mode::HalfEven>();
    check_div_boundaries::<mode::HalfAway>();
}

/// Exponent saturation must keep flowing through the exact path's error
/// semantics through the public API.
#[test]
fn test_div_exponent_overflow() {
    let aw = pattern_words(words_for_precision(2400, 2), 33);
    // a huge exponent difference: (MAX − 5) − (−10) overflows on subtraction
    let a = Repr::<2>::new(significand(&aw), isize::MAX - 5);
    let b = Repr::<2>::new(significand(&pattern_words(words_for_precision(2400, 2), 45)), -10);
    let ctx = Context::<mode::HalfEven>::new(2400);
    assert_eq!(ctx.div(&a, &b), Err(FpError::Overflow(Sign::Positive)));

    let c = Repr::<2>::new(significand(&aw), isize::MIN + 5);
    let d = Repr::<2>::new(significand(&pattern_words(words_for_precision(2400, 2), 55)), 10);
    assert_eq!(ctx.div(&c, &d), Err(FpError::Underflow(Sign::Positive)));
}

/// Sparse power-of-two patterns give products with long zero runs around
/// the rounding split. The chained cubic window can collapse its certified
/// margin on such operands — declining to the exact product is then the
/// correct outcome — all three operations are checked through the public API.
#[test]
fn test_short_mul_sparse_patterns() {
    let len = scale(64);
    for &k in &[1usize, 31, 62] {
        let mid = if k == 62 { len - 2 } else { scale(k) };
        let mut aw = vec![0 as Word; len];
        aw[0] = 1;
        aw[len - 1] = 1;
        let mut bw = vec![0 as Word; len];
        bw[0] = 1; // odd low word: keep normalization from shrinking the significand
        bw[mid] = 3;
        bw[len - 1] = 5;
        let (a2, b2) = (Repr::<2>::new(significand(&aw), 0), Repr::<2>::new(significand(&bw), 0));
        for &p in &[2usize, 20, 50, 100, 500] {
            check_mul::<mode::HalfEven, 2>(p, &a2, &b2);
            check_mul::<mode::Down, 2>(p, &a2, &b2);
            check_sqr::<mode::HalfEven, 2>(p, &b2);
            check_sqr::<mode::HalfAway, 2>(p, &a2);
            check_products::<mode::HalfEven, 2>(p, &a2, &b2, &b2);
            check_products::<mode::Down, 2>(p, &a2, &b2, &a2);
        }
    }
}

/// Base-10 significands with trailing zero words: the dropped part of the
/// product is exactly zero. Such operands collapse the chained cubic
/// window's certified margin (and with it the engagement). All three
/// operations are checked through the public API.
#[test]
fn test_short_mul_exact_windows() {
    let len = scale(64);
    let mut aw = vec![0 as Word; len];
    aw[len - 1] = 7;
    let mut bw = vec![0 as Word; len];
    bw[len - 1] = 9;
    let (a10, b10) = (Repr::<10>::new(significand(&aw), 0), Repr::<10>::new(significand(&bw), 0));
    for &p in &[2usize, 50, 500] {
        check_mul::<mode::HalfEven, 10>(p, &a10, &b10);
        check_mul::<mode::HalfAway, 10>(p, &a10, &b10);
        check_products::<mode::HalfEven, 10>(p, &a10, &b10, &a10);
        check_products::<mode::Zero, 10>(p, &a10, &b10, &b10);
    }
}

/// Exponent saturation must keep flowing through the exact path's error
/// semantics through the public API.
#[test]
fn test_mul_exponent_overflow() {
    let aw = pattern_words(scale(64), 0x9999_9999_9999_999b);
    let a = Repr::<2>::new(significand(&aw), isize::MAX - 5);
    let b = Repr::<2>::new(significand(&aw), 10);
    let ctx = Context::<mode::HalfEven>::new(50);
    assert_eq!(ctx.mul(&a, &b), Err(FpError::Overflow(Positive)));

    let c = Repr::<2>::new(significand(&aw), isize::MIN + 5);
    let d = Repr::<2>::new(significand(&aw), -10);
    assert_eq!(ctx.mul(&c, &d), Err(FpError::Underflow(Positive)));
}

/// A power-of-two base whose per-digit bit count does not divide the
/// dropped bit count (base 8 on 64-bit words: 64·dropped mod 3 ≠ 0) must
/// pad the remainder bits onto the significand instead of folding them
/// into the exponent by floor division (which mis-scales the value).
#[test]
fn test_short_mul_power_of_two_base_exponent_folding() {
    let aw = pattern_words(scale(64), 0x0cab_ba6e_5032_51f9);
    let bw = pattern_words(scale(64), 0x5eed_5eed_5eed_5eed);
    let p = 100; // octal digits; the window drops 121 words ≡ 1 mod 3
    let a8 = Repr::<8>::new(significand(&aw), 3);
    let b8 = Repr::<8>::new(significand(&bw), -2);
    check_mul::<mode::HalfEven, 8>(p, &a8, &b8);
    check_sqr::<mode::HalfEven, 8>(p, &a8);
    check_cubic::<mode::HalfEven, 8>(p, &a8);
}

/// Sweeping the exponent sum across the finite-range boundary: whenever
/// the unlimited-precision oracle saturates to the infinity, the public
/// operation must report the corresponding error — including the case
/// where the fast path's `normalize` bumps a checked exponent exactly
/// onto the sentinel (which the fast path's own `checked_add` cannot
/// see).
#[test]
fn test_short_mul_exponent_saturation_parity() {
    let aw = pattern_words(scale(64), 0x7b1f_2e3d_4c5b_6a79);
    let p = 50usize;
    let ctx = Context::<mode::HalfEven>::new(p);
    let oracle_ctx = Context::<mode::HalfEven>::new(0);
    let b = Repr::<2>::new(significand(&aw), 0);
    for delta in 0..(scale(8192) as isize) {
        let a = Repr::<2>::new(significand(&aw), isize::MAX - delta);
        let got = ctx.mul(&a, &b);
        match oracle_ctx.mul(&a, &b) {
            // The unlimited product itself can saturate in `normalize`
            // (its trailing-zero fold crosses the sentinel): both paths
            // must report overflow.
            Err(FpError::Overflow(_)) => {
                assert!(
                    matches!(got, Err(FpError::Overflow(_))),
                    "delta={delta}: expected overflow, got {:?}",
                    got.map(|v| v.value().repr().clone())
                );
            }
            Err(e) => panic!("delta={delta}: unexpected oracle error {e:?}"),
            Ok(want) => {
                let want = want.value().with_precision(p);
                let want_repr = match &want {
                    Exact(v) | Inexact(v, _) => v.repr(),
                };
                if want_repr.is_infinite() {
                    assert!(
                        matches!(got, Err(FpError::Overflow(_))),
                        "delta={delta}: expected overflow, got {:?}",
                        got.map(|v| v.value().repr().clone())
                    );
                } else {
                    let got = got.expect("finite oracle but the public op errored");
                    assert_eq!(got.value().repr(), want_repr, "delta={delta}");
                }
            }
        }
    }
}

/// Rounding can push the *result* out of the finite exponent range even
/// when the exact intermediate is finite: a wide significand rounded to a
/// small precision needs `exponent + shift` beyond `isize::MAX`. This
/// used to panic in debug builds and wrap the exponent in release builds.
#[test]
fn test_round_exponent_saturation() {
    let aw = pattern_words(scale(64), 0x5a5a_5a5a_5a5a_5a5b);
    let p = 50usize;
    let ctx = Context::<mode::HalfEven>::new(p);

    // Multiplication: the exact product is finite (exponent well below
    // the sentinel), but rounding 128 words down to 50 digits overflows.
    let a = Repr::<2>::new(significand(&aw), isize::MAX - 7000);
    let b = Repr::<2>::new(significand(&aw), 10);
    assert_eq!(ctx.mul(&a, &b), Err(FpError::Overflow(Positive)));
    let na = Repr::<2>::new(-significand(&aw), isize::MAX - 7000);
    assert_eq!(ctx.mul(&na, &b), Err(FpError::Overflow(Negative)));

    // Square and cubic reach the same state through their exact paths.
    assert_eq!(ctx.sqr(&a), Err(FpError::Overflow(Positive)));
    assert_eq!(ctx.cubic(&na), Err(FpError::Overflow(Negative)));

    // Addition: the aligned sum keeps the huge exponent and its wide
    // significand rounds past the range.
    let hi = Repr::<2>::new(significand(&aw), isize::MAX - 1000);
    assert_eq!(ctx.add(&hi, &b), Err(FpError::Overflow(Positive)));
    assert_eq!(ctx.sub(&hi, &b), Err(FpError::Overflow(Positive)));

    // `with_precision` has no error channel: the value saturates to the
    // infinity sentinel instead of panicking.
    let exact = FBig::<mode::HalfEven, 2>::from_parts(significand(&aw), isize::MAX - 1000);
    let rounded = exact.with_precision(p);
    assert!(rounded.value().repr().is_infinite());
}
